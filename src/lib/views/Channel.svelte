<script lang="ts">
  import ChannelList from "$lib/views/channel/ChannelList.svelte";
  import ChannelProcess from "$lib/views/channel/ChannelProcess.svelte";
  import { session } from "$lib/stores/session.svelte";

  const form = session.channel;
</script>

<!-- Daftar channel dan halaman prosesnya dipisah: baris tabel membuka prosesnya. -->
{#if form.channelId}
  <ChannelProcess />
{:else}
  <ChannelList />
{/if}
