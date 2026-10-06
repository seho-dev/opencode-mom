#!/usr/bin/perl
use strict;
use warnings;
use Fcntl qw(:DEFAULT :mode O_NOFOLLOW);
use File::Temp qw(tempdir);
use FindBin qw($Bin);

die "Run this regression without root privileges\n" if $> == 0;
umask 0077;
my $temporary = tempdir('lid-bootstrap-XXXXXX',
    DIR => '/private/var/folders/8w/1817sjk90j75l5r2ppktkj700000gn/T/opencode', CLEANUP => 1);
my $control = "$temporary/control";
my $owner = $>;
open(my $bootstrap, '<', "$Bin/macos_bootstrap.pl") or die "Cannot read bootstrap: $!\n";
my $source = do { local $/; <$bootstrap> };
close($bootstrap) or die "Cannot close bootstrap: $!\n";
my ($function) = $source =~ /^(sub secure_directory \{\n.*?^\})/ms;
die "Cannot extract secure_directory\n" unless defined($function);
my $predicates = $function =~ s/(\$(?:before|opened)\[4\]) == 0/$1 == \$owner/g;
die "Cannot adapt root-owner predicate\n" unless $predicates;
eval $function;
die $@ if $@;

sub accepts {
    my ($path, $mode) = @_;
    my @before = lstat($path);
    my $directory = secure_directory($path, $mode);
    my @opened = stat($directory);
    my @after = lstat($path);
    die "Wrong accepted directory: $path\n"
        unless @opened && @after && S_ISDIR($opened[2]) && $opened[4] == $>
            && ($opened[2] & 07777) == $mode && ($after[2] & 07777) == $mode
            && $opened[0] == $after[0] && $opened[1] == $after[1]
            && (!@before || ($before[0] == $opened[0] && $before[1] == $opened[1]));
    close($directory) or die "Cannot close directory: $!\n";
}

sub rejects {
    my ($path, $mode) = @_;
    my @before = lstat($path);
    my $accepted = eval { my $directory = secure_directory($path, $mode); close($directory); 1 };
    die "Accepted unsafe directory: $path\n" if $accepted;
    my @after = lstat($path);
    die "Changed rejected directory: $path\n"
        unless @before && @after && $before[0] == $after[0] && $before[1] == $after[1]
            && $before[2] == $after[2] && $before[4] == $after[4];
}

accepts($control, 0711);
chmod(0700, $control) == 1 or die "Cannot prepare legacy mode: $!\n";
accepts($control, 0711);
accepts($control, 0711);
for my $mode (0777, 0755, 0701, 01711) {
    chmod($mode, $control) == 1 or die "Cannot prepare unsafe mode: $!\n";
    rejects($control, 0711);
}
mkdir("$temporary/other-control", 0700) or die "Cannot prepare other path: $!\n";
rejects("$temporary/other-control", 0711);

rmdir($control) or die "Cannot remove temporary control: $!\n";
mkdir("$temporary/target", 0700) or die "Cannot prepare symlink target: $!\n";
symlink("$temporary/target", $control) or die "Cannot prepare symlink: $!\n";
rejects($control, 0711);
die "Changed symlink target\n" unless ((lstat("$temporary/target"))[2] & 07777) == 0700;
unlink($control) or die "Cannot remove temporary symlink: $!\n";
mkdir($control, 0700) or die "Cannot prepare owner check: $!\n";
$owner = $> + 1;
for my $mode (0700, 0711) {
    chmod($mode, $control) == 1 or die "Cannot prepare owner mode: $!\n";
    rejects($control, 0711);
}
$owner = $>;
accepts("$temporary/root", 0700);
accepts("$temporary/root", 0700);
accepts("$temporary/root/stage", 0700);
accepts("$temporary/root/stage", 0700);
print "PASS: masked creation, legacy repair, correct mode, unsafe modes, fixed-path restriction, symlink, owner, root/stage\n";
