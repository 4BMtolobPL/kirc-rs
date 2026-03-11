import { SvelteMap } from "svelte/reactivity";
import type { ChannelId, ChatMessage, Server, ServerId } from "../types/kirc.svelte";

export class IrcStore {
  servers = $state(new SvelteMap<ServerId, Server>());
  messages = $state(new SvelteMap<ChannelId, ChatMessage[]>());
  currentServerId = $state<ServerId | null>(null);
  currentChannelId = $state<ChannelId | null>(null);

  currentServer = $derived.by(() => {
    if (!this.currentServerId) return null;
    return this.servers.get(this.currentServerId) ?? null;
  });

  currentChannel = $derived.by(() => {
    if (!this.currentServerId || !this.currentChannelId) return null;
    return this.servers.get(this.currentServerId)?.channels.get(this.currentChannelId) ?? null;
  });

  currentMessage = $derived.by(() => {
    if (!this.currentServerId || !this.currentChannelId) return null;
    return this.messages.get(this.currentChannelId) ?? null;
  });

  currentServerNickname = $derived.by(() => {
    return this.currentServer?.nickname ?? null;
  });

  isLocked = $derived.by(() => {
    return this.currentChannel?.locked ?? true;
  });

  serverUnread = $derived.by(() => {
    const result = new Map<string, number>();
    for (const [serverId, server] of this.servers) {
      let total = 0;
      for (const channel of server.channels.values()) {
        total += channel.unread;
      }
      result.set(serverId, total);
    }
    return result;
  });

  constructor() {}
}

export const ircStore = new IrcStore();
