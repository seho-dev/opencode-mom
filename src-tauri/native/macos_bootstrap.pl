BEGIN {
    @INC = grep { m{\A/System/Library/Perl/} } @INC;
}
use strict;
use warnings;
use Fcntl qw(:DEFAULT :flock :mode F_SETFD O_NOFOLLOW O_NONBLOCK);
use POSIX qw(dup2 setsid _exit);

%ENV = (PATH => '/usr/bin:/bin:/usr/sbin:/sbin', LANG => 'C', LC_ALL => 'C');
umask 0077;
die "Authorization required\n" unless $> == 0;
die "Invalid launch arguments\n" unless @ARGV == 5;
my ($source, $uid, $pid, $token, $digest) = @ARGV;
($source) = $source =~ m{\A(/[^\x00-\x1f\x7f]{1,2048})\z} or die "Invalid source\n";
($uid) = $uid =~ /\A([1-9][0-9]{0,9})\z/ or die "Invalid user\n";
($pid) = $pid =~ /\A([1-9][0-9]{0,9})\z/ or die "Invalid process\n";
die "Invalid identity\n" if $uid > 4294967294 || $pid < 2 || $pid > 2147483647;
($token) = $token =~ /\A([0-9a-f]{64})\z/ or die "Invalid token\n";
($digest) = $digest =~ /\A([0-9a-f]{64})\z/ or die "Invalid digest\n";

my $root = '/private/var/db/opencode-mom-lid';
my $control = '/private/var/run/opencode-mom-lid';
my $stage;
END {
    if (defined($stage)) {
        unlink "$root/$stage/helper";
        rmdir "$root/$stage";
    }
}
sub secure_directory {
    my ($path, $mode) = @_;
    my $allow_masked = $path eq $control && $mode == 0711;
    my @before = lstat($path);
    if (!@before) {
        mkdir($path, $mode) or die "Cannot create trusted directory\n";
        @before = lstat($path);
    }
    die "Unsafe trusted directory\n"
        unless @before && S_ISDIR($before[2]) && $before[4] == 0
            && (($before[2] & 07777) == $mode || ($allow_masked && ($before[2] & 07777) == 0700));
    sysopen(my $directory, $path, O_RDONLY | O_NOFOLLOW) or die "Cannot open trusted directory\n";
    my @opened = stat($directory);
    die "Directory changed\n"
        unless @opened && $opened[0] == $before[0] && $opened[1] == $before[1]
            && S_ISDIR($opened[2]) && $opened[4] == 0
            && (($opened[2] & 07777) == $mode || ($allow_masked && ($opened[2] & 07777) == 0700));
    if ($allow_masked && ($opened[2] & 07777) == 0700) {
        chmod($mode, $directory) == 1 or die "Cannot normalize trusted directory\n";
    }
    @opened = stat($directory);
    die "Unsafe trusted directory\n"
        unless @opened && $opened[0] == $before[0] && $opened[1] == $before[1]
            && S_ISDIR($opened[2]) && $opened[4] == 0 && ($opened[2] & 07777) == $mode;
    return $directory;
}
my $directory = secure_directory($root, 0700);
my $control_directory = secure_directory($control, 0711);
chdir($directory) or die "Cannot enter trusted directory\n";

my $lock;
if (!sysopen($lock, 'lock', O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW, 0600)) {
    sysopen($lock, 'lock', O_RDWR | O_NOFOLLOW) or die "Cannot open trusted lock\n";
}
my @lock_stat = stat($lock);
die "Unsafe trusted lock\n"
    unless @lock_stat && S_ISREG($lock_stat[2]) && $lock_stat[4] == 0
        && ($lock_stat[2] & 07777) == 0600 && $lock_stat[3] == 1;
flock($lock, LOCK_EX | LOCK_NB) or die "Another authorized helper is running\n";

sysopen(my $input, $source, O_RDONLY | O_NOFOLLOW | O_NONBLOCK) or die "Cannot read helper bytes\n";
my @input_stat = stat($input);
die "Unsafe helper source\n"
    unless @input_stat && S_ISREG($input_stat[2]) && $input_stat[4] == $uid
        && $input_stat[3] == 1 && $input_stat[7] > 0 && $input_stat[7] <= 16777216;
$stage = 'stage-' . $token;
mkdir($stage, 0700) or die "Cannot create exclusive root staging\n";
my $stage_directory = secure_directory($stage, 0700);
chdir($stage_directory) or die "Cannot enter root staging\n";
sysopen(my $output, 'helper', O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0700)
    or die "Cannot stage helper\n";
my $total = 0;
while (1) {
    my $count = sysread($input, my $buffer, 65536);
    die "Cannot read helper bytes\n" unless defined($count);
    last if $count == 0;
    $total += $count;
    die "Helper bytes exceeded limit\n" if $total > 16777216;
    my $offset = 0;
    while ($offset < $count) {
        my $written = syswrite($output, $buffer, $count - $offset, $offset);
        die "Cannot stage helper bytes\n" unless defined($written) && $written > 0;
        $offset += $written;
    }
}
close($input) or die "Cannot close source\n";
close($output) or die "Cannot close staged helper\n";
my $hash_code = 'BEGIN { @INC = grep { m{\A/System/Library/Perl/} } @INC; } do "/usr/bin/shasum"; die $@ if $@;';
my $actual;
eval {
    local $SIG{ALRM} = sub { die "Digest verification timed out\n" };
    alarm 10;
    open(my $hash, '-|', '/usr/bin/perl', '-T', '-e', $hash_code, '--', '-a', '256', '--', 'helper')
        or die "Cannot verify helper\n";
    local $/;
    $actual = <$hash>;
    close($hash) or die "Helper verification failed\n";
    alarm 0;
};
alarm 0;
die "Helper digest verification failed\n" if $@;
die "Helper bytes do not match this build\n"
    unless defined($actual) && $actual =~ /\A\Q$digest\E  helper\n\z/;
eval {
    local $SIG{ALRM} = sub { die "Executable verification timed out\n" };
    alarm 10;
    system('/usr/bin/codesign', '--verify', '--strict', 'helper') == 0
        or die "Helper executable integrity verification failed\n";
    alarm 0;
};
alarm 0;
die "Helper executable verification failed\n" if $@;

pipe(my $ready_read, my $ready_write) or die "Cannot create launch acknowledgment\n";
my $child = fork();
die "Cannot launch helper\n" unless defined($child);
if ($child == 0) {
    close($ready_read);
    setsid() >= 0 or _exit(1);
    sysopen(my $null, '/dev/null', O_RDWR) or _exit(1);
    my $lock_copy;
    my $ready_copy;
    open($lock_copy, '>&', $lock) or _exit(1);
    open($ready_copy, '>&', $ready_write) or _exit(1);
    dup2(fileno($null), 0) >= 0 or _exit(1);
    dup2(fileno($null), 1) >= 0 or _exit(1);
    dup2(fileno($null), 2) >= 0 or _exit(1);
    dup2(fileno($lock_copy), 3) >= 0 or _exit(1);
    dup2(fileno($ready_copy), 4) >= 0 or _exit(1);
    # Keep only the trusted lock and bounded launch acknowledgment across exec.
    my @inherited;
    for my $fd (3, 4) {
        open(my $handle, '+<&=', $fd) or _exit(1);
        fcntl($handle, F_SETFD, 0) or _exit(1);
        push @inherited, $handle;
    }
    for my $handle ($directory, $control_directory, $lock, $ready_write, $lock_copy, $ready_copy, $null) {
        next unless defined(fileno($handle));
        next if fileno($handle) == 3 || fileno($handle) == 4;
        close($handle);
    }
    exec { './helper' } './helper', $uid, $pid, $token;
    _exit(1);
}
close($ready_write);
# The detached helper owns the flock; osascript returns after READY, not helper exit.
my $response = '';
eval {
    local $SIG{ALRM} = sub { die "Launch timed out\n" };
    alarm 15;
    while (length($response) < 6) {
        my $count = sysread($ready_read, my $buffer, 6 - length($response));
        die "Helper launch failed\n" unless defined($count) && $count > 0;
        $response .= $buffer;
        last if $response =~ /\n/;
    }
    alarm 0;
};
alarm 0;
die "Helper launch failed\n" if $@ || $response ne "READY\n";
close($ready_read);
unlink('helper') or die "Cannot remove staged executable\n";
chdir($root) or die "Cannot leave root staging\n";
rmdir($stage) or die "Cannot remove root staging\n";
print "READY\n";
