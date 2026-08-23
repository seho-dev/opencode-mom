<script lang="ts">
  let { colspan, message }: { colspan: number; message: string } = $props();
</script>

<tr><td {colspan} class="empty-table-row">{message}</td></tr>
