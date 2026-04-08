<script lang="ts">
    import {ircStore} from "../stores/irc.svelte";
    import {MessageType} from "../types/kirc.svelte";

    // oxlint-disable-next-line no-unassigned-vars
    let container: HTMLDivElement;
    let autoScroll = $state<boolean>(true);

    const onScroll = () => {
        autoScroll = container.scrollHeight - container.scrollTop - container.clientHeight < 50;
    }

    $effect(() => {
        if (autoScroll && ircStore.currentMessage) {
            container.scrollTop = container.scrollHeight;
        }
    });
</script>


<div bind:this={container} class="flex-1 overflow-y-auto p-3" onscroll={onScroll}>
    {#each ircStore.currentMessage ?? [] as msg}
        {#if msg.type === MessageType.USER}
            <div class="mb-1">
                <span class="font-semibold">{(ircStore.currentServerNickname && ircStore.currentServerNickname === msg.nickname) ? `< ${msg.nickname}>` : `<@${msg.nickname}>`}</span>
                <span class="ml-1 whitespace-pre-wrap">{msg.content}</span>
            </div>
        {/if}
        {#if msg.type === MessageType.SYSTEM}
            <div class="mb-1">
                <span class="font-semibold text-gray-500">System</span>
                <span class="ml-1 whitespace-pre-wrap text-gray-500">{msg.content}</span>
            </div>
        {/if}
    {/each}
</div>