import { SvelteMap } from "svelte/reactivity";
import type { Channel, ChannelId, ChatMessage, Server, ServerId } from "../types/kirc.svelte";

export class IrcStore {
  servers = $state(new SvelteMap<ServerId, Server>());
  channels = $state(new SvelteMap<ChannelId, Channel>());
  messages = $state(new SvelteMap<ChannelId, ChatMessage[]>());
  currentServerId = $state<ServerId | null>(null);
  currentChannelId = $state<ChannelId | null>(null);
  nickErrors = $state(new SvelteMap<ServerId, string>());
  nickSuccess = $state(new SvelteMap<ServerId, string>());

  currentServer = $derived.by(() => {
    if (!this.currentServerId) return null;
    return this.servers.get(this.currentServerId) ?? null;
  });

  currentChannel = $derived.by(() => {
    if (!this.currentChannelId) return null;
    return this.channels.get(this.currentChannelId) ?? null;
  });

  currentMessage = $derived.by(() => {
    if (!this.currentChannelId) return null;
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
    for (const serverId of this.servers.keys()) {
      result.set(serverId, 0);
    }

    for (const channel of this.channels.values()) {
      const current = result.get(channel.serverId) ?? 0;
      result.set(channel.serverId, current + channel.unread);
    }
    return result;
  });

  constructor() {}
}

export const ircStore = new IrcStore();
