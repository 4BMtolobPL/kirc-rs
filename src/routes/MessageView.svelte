<script lang="ts">
    import {ircStore} from "../stores/irc.svelte";
    import {MessageType} from "../types/kirc.svelte";
    import {tick} from "svelte";
    import UnreadDivider from "./UnreadDivider.svelte";
    import {ircService} from "../services/ircService";

    // oxlint-disable-next-line no-unassigned-vars
    let container: HTMLDivElement;
    let isBottom = $state<boolean>(true);

    $effect(() => {
        if (isBottom && ircStore.currentMessage) {
            container.scrollTop = container.scrollHeight;
        }
    });

    $effect(() => {
        if (ircStore.currentChannelId) {
            restoreScroll();
        }
    })

    const onScroll = () => {
        console.log("onScroll", isBottom);
        isBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 10;

        if (isBottom && ircStore.currentChannelId) {
            ircService.updateChannelLastReadMessageId(ircStore.currentChannelId, ircStore.currentMessage?.[ircStore.currentMessage.length - 1]?.id)
        }
    }

    const restoreScroll = async () => {
        await tick();

        if (!ircStore.currentChannel) return;
        if (!ircStore.currentChannel.lastReadMessageId) return;
        const lastReadMessageId = ircStore.currentChannel.lastReadMessageId;

        const element = document.getElementById(lastReadMessageId);
        if (element) {
            element.scrollIntoView({behavior: "instant", block: "start"/*, inline: "nearest"*/});
        }
    }
</script>


<div bind:this={container} class="flex-1 overflow-y-auto p-3" onscroll={onScroll}>
    {#each ircStore.currentMessage ?? [] as msg, index (msg.id)}
        {#if msg.type === MessageType.USER}
            <div class="mb-1" id={msg.id}>
                <span class="font-semibold">{(ircStore.currentServerNickname && ircStore.currentServerNickname === msg.nickname) ? `< ${msg.nickname}>` : `<@${msg.nickname}>`}</span>
                <span class="ml-1 whitespace-pre-wrap">{msg.content}</span>
            </div>
        {:else if msg.type === MessageType.SYSTEM}
            <div class="mb-1">
                <span class="font-semibold text-gray-500">System</span>
                <span class="ml-1 whitespace-pre-wrap text-gray-500">{msg.content}</span>
            </div>
        {/if}
        {#if msg.id === ircStore.currentChannel?.lastReadMessageId && index !== ((ircStore.currentMessage?.length ?? 0) - 1)}
            <UnreadDivider />
        {/if}
    {/each}
</div>