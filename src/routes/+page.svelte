<script lang="ts">
    import {onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/core";
    import ServerModal from "./ServerModal.svelte";
    import ChannelJoinModal from "./ChannelJoinModal.svelte";
    import MessageView from "./MessageView.svelte";
    import {ircStore} from "../stores/irc.svelte";
    import {ircService} from "../services/ircService";
    import type {ChannelId, ServerId} from "../types/kirc.svelte.ts";
    import ChangeNicknameModal from "./ChangeNicknameModal.svelte";

    let showChannelModal = $state<boolean>(false);
    let msgInput = $state<string>("");

    let showServerModal = $state<boolean>(false);
    let showChangeNickModal = $state<boolean>(false)

    let channelContextMenu = $state<{
        visible: boolean,
        x: number,
        y: number,
        serverId: ServerId | null,
        channelId: string | null,
        channelName: string | null,
    }>({
        visible: false,
        x: 0,
        y: 0,
        serverId: null,
        channelId: null,
        channelName: null,
    });
    let serverContextMenu = $state<{
        visible: boolean,
        x: number,
        y: number,
        serverId: ServerId | null
    }>({
        visible: false,
        x: 0,
        y: 0,
        serverId: null,
    });
    window.addEventListener("click", () => {
        channelContextMenu.visible = false;
        serverContextMenu.visible = false;
    });

    onMount(async () => {
        await ircService.initialize();
    });

    const sendMessage = async (e: Event): Promise<void> => {
        e.preventDefault();

        if (!ircStore.currentServerId || !ircStore.currentChannel) return;

        if (msgInput.trim() === "") return;

        await invoke("send_message", {
            serverId: ircStore.currentServerId,
            target: ircStore.currentChannel.name,
            message: msgInput
        });

        msgInput = "";
    }

    const selectServer = (serverId: ServerId) => {
        ircService.setCurrentServer(serverId);
        ircService.setCurrentChannel(null);
    }

    const selectChannel = (channelId: ChannelId) => {
        ircService.setCurrentChannel(channelId);
    }

    const openChannelModal = () => {
        showChannelModal = true;
    }

    const openServerModal = () => {
        showServerModal = true;
    }

    const toggleLock = () => {
        const serverId = ircStore.currentServerId;
        const channel = ircStore.currentChannelId;

        if (ircStore.isLocked) {
            invoke("unlock_channel", {payload: {serverId, channel}});
        } else {
            invoke("lock_channel", {payload: {serverId, channel}});
        }

        msgInput = "";
    }

    const leaveChannel = () => {
        invoke("leave_channel", {
            payload: {
                serverId: channelContextMenu.serverId,
                channel: channelContextMenu.channelName
            }
        });
    }

    const disconnectServer = () => {
        invoke("disconnect_server", {serverId: serverContextMenu.serverId});
    }

    const changeNickname = () => {
        showChangeNickModal = true
    }
</script>

<div class="w-dvw h-dvh flex bg-neutral-100 text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100">
    <!-- 좌측 패널 HTML 구조 -->
    <aside class="w-56 shrink-0 border-r border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900">
        <div class="p-2 text-sm font-semibold">Servers</div>
        <ul class="space-y-1 px-2">
            {#each ircStore.servers as [serverId, server] (serverId)}
                {@const isSelected = serverId === ircStore.currentServerId}
                {@const unread = ircStore.serverUnread.get(serverId) ?? 0}
                <li>
                    <!-- Server Row -->
                    <button class="w-full flex items-center justify-between rounded px-3 py-2 {isSelected ? 'bg-neutral-200 dark:bg-neutral-700' : 'hover:bg-neutral-100 dark:hover:bg-neutral-800'}"
                            onclick={() => selectServer(serverId)}
                            oncontextmenu={(e) => {
                                e.preventDefault();
                                serverContextMenu = {
                                    visible: true,
                                    x: e.clientX,
                                    y: e.clientY,
                                    serverId: serverId,
                                };
                    }}>
                        <!-- Left -->
                        <span class="truncate">{server.name}</span>

                        <!-- Right -->
                        <span class="flex items-center gap-2 shrink-0">
                            {#if !isSelected && unread > 0}
                                  <span class="min-w-5 px-2 py-0.5 text-xs font-medium bg-red-500 text-white rounded-full text-center">
                                      {unread > 99 ? '99+' : unread}
                                  </span>
                            {/if}

                            <!-- Status Dot -->
                            <span class="h-2 w-2 rounded-full {server.status === 'connected' ? 'bg-green-500' : (server.status === 'connecting' || server.status === 'registering') ? 'bg-yellow-500' : 'bg-red-500'}"></span>
                        </span>
                    </button>

                    <!-- Channel List -->
                    {#if serverId === ircStore.currentServerId}
                        <ul class="ml-4 mt-1 space-y-1 text-sm">
                            {#each ircStore.channels.entries().filter(([_channelId, channel]) => channel.serverId === serverId) as [channelId, channel] (channelId)}
                                <li oncontextmenu={(e) => {
                                    e.preventDefault();
                                    channelContextMenu = {
                                        visible: true,
                                        x: e.clientX,
                                        y: e.clientY,
                                        serverId: serverId,
                                        channelId: channelId,
                                        channelName: channel.name,
                                    };
                                }}>
                                    <button class="w-full cursor-pointer rounded px-2 py-1 {channelId === ircStore.currentChannelId ? 'bg-neutral-300 dark:bg-neutral-600' : 'hover:bg-neutral-200 dark:hover:bg-neutral-700'}"
                                            onclick={() => selectChannel(channelId)}>
                                        <span class="flex items-center gap-1">
                                            {channel.name}
                                            {#if channel.unread > 0}
                                                <span class="rounded-full bg-red-500 px-1.5 text-xs text-white">
                                                    {channel.unread > 99 ? '99+' : channel.unread}
                                                </span>
                                            {/if}
                                        </span>
                                    </button>
                                </li>
                            {/each}

                            <!-- Channel Add -->
                            <li>
                                <button class="w-full cursor-pointer flex items-center justify-between rounded px-2 py-1 text-neutral-500 hover:bg-neutral-200 dark:hover:bg-neutral-700"
                                        onclick={() => openChannelModal()}>+ 채널 추가
                                </button>
                            </li>
                        </ul>
                    {/if}
                </li>
            {/each}

            <!-- Server Add -->
            <li>
                <button class="w-full cursor-pointer flex items-center justify-between mt-2 rounded px-2 py-1 text-sm text-neutral-600 hover:bg-neutral-200 dark:text-neutral-400 dark:hover:bg-neutral-700"
                        onclick={() => openServerModal()}>+ 서버 추가
                </button>
            </li>
        </ul>
    </aside>

    <!-- 우측 메인 영역 -->
    <main class="flex flex-col flex-1">
        <!-- 메시지 목록 -->
        <MessageView></MessageView>

        <!-- 입력 영역 -->
        <section class="border-t border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-900 p-3">
            <form class="flex gap-2" onsubmit={sendMessage}>
                <button class="text-sm px-2 py-1 rounded hover:bg-neutral-200 dark:hover:bg-neutral-700"
                        onclick={toggleLock} type="button">{ircStore.isLocked ? "🔒" : "🔓"}</button>
                <input bind:value={msgInput}
                       class="flex-1 rounded px-3 py-2 text-sm bg-white dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-700 focus:outline-none focus:ring-1 focus:ring-sky-500"
                       disabled={ircStore.isLocked}
                       placeholder={ircStore.isLocked ? "Channel is locked" : "Type a message"}
                       type="text"/>
                <button class="rounded px-4 py-2 text-sm bg-sky-600 text-white hover:bg-sky-500 active:bg-sky-700 disabled:opacity-40"
                        disabled={ircStore.isLocked} type="submit">Send
                </button>
            </form>
        </section>
    </main>
</div>

{#if showChannelModal}
    <ChannelJoinModal bind:showChannelModal></ChannelJoinModal>
{/if}
{#if showServerModal}
    <ServerModal bind:showServerModal></ServerModal>
{/if}

{#if serverContextMenu.visible }
    <div class="fixed z-50 rounded bg-neutral-800 text-white shadow"
         style="left: {channelContextMenu.x}px; top: {channelContextMenu.y}px;">
        <button>Connect</button>
        <button onclick={disconnectServer}>Disconnect</button>
        <button>Copy Server Name</button>
        <button onclick={changeNickname}>Change Nickname</button>
    </div>
{/if}
{#if channelContextMenu.visible }
    <div class="fixed z-50 rounded bg-neutral-800 text-white shadow"
         style="left: {channelContextMenu.x}px; top: {channelContextMenu.y}px;">
        <button>Join</button>
        <button onclick={leaveChannel}>Leave</button>
        <button>Copy Channel Name</button>
    </div>
{/if}

<ChangeNicknameModal bind:showModal={showChangeNickModal} serverId={serverContextMenu.serverId}/>

<style>
</style>