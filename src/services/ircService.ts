import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ircStore } from "../stores/irc.svelte";
import { SvelteMap, SvelteSet } from "svelte/reactivity";
import { type ChatMessage, type IrcServerStatus, MessageType } from "../types/kirc.svelte";
import type { ChannelLockChangedEvent, UiEventPayload } from "../types/payloads.svelte";

export class IrcService {
  async initialize() {
    await invoke("init_servers");
    const initialServers = await invoke<any[]>("get_servers");

    initialServers.forEach((s) => {
      const channelsMap = new SvelteMap();
      s.channels.forEach((ch: any) => {
        channelsMap.set(ch.name, {
          name: ch.name,
          messages: [],
          users: new SvelteSet(),
          unread: 0,
          locked: ch.locked,
        });
      });

      ircStore.servers.set(s.id, {
        ...s,
        channels: channelsMap,
        serverMessages: [],
      });
    });

    this.setupEventListeners();
  }

  private setupEventListeners() {
    listen<UiEventPayload>("kirc:event", (event) => {
      const payload = event.payload;
      console.log("kirc:event", payload);

      switch (payload.type) {
        case "UserMessage": {
          this.ensureChannel(payload.server_id, payload.channel);
          this.addMessage(payload.server_id, payload.channel, {
            type: MessageType.USER,
            id: crypto.randomUUID(),
            nickname: payload.nick,
            content: payload.content,
            timestamp: payload.timestamp,
          });
          break;
        }
        case "Join": {
          this.ensureChannel(payload.server_id, payload.channel);
          const server = ircStore.servers.get(payload.server_id);
          if (server) {
            const channel = server.channels.get(payload.channel);
            if (channel) {
              channel.users.add(payload.nick);
              channel.messages = [
                ...channel.messages,
                {
                  type: MessageType.SYSTEM,
                  id: crypto.randomUUID(),
                  content: `${payload.nick} joined the channel`,
                  timestamp: Date.now(),
                },
              ];

              if (payload.nick === server.nickname) {
                ircStore.currentServerId = payload.server_id;
                ircStore.currentChannelName = payload.channel;
              }
            }
          }
          break;
        }
        case "Part": {
          const server = ircStore.servers.get(payload.server_id);
          if (server) {
            if (payload.nick === server.nickname) {
              if (ircStore.currentChannelName === payload.channel) {
                ircStore.currentChannelName = null;
              }
              server.channels.delete(payload.channel);
            } else {
              const channel = server.channels.get(payload.channel);
              if (channel) {
                channel.users.delete(payload.nick);
                channel.messages = [
                  ...channel.messages,
                  {
                    type: MessageType.SYSTEM,
                    id: crypto.randomUUID(),
                    content: `${payload.nick} left the channel`,
                    timestamp: Date.now(),
                  },
                ];
              }
            }
          }
          break;
        }
        case "Quit": {
          const server = ircStore.servers.get(payload.server_id);
          if (server) {
            for (const channel of server.channels.values()) {
              if (channel.users.has(payload.nick)) {
                channel.users.delete(payload.nick);
                channel.messages = [
                  ...channel.messages,
                  {
                    type: MessageType.SYSTEM,
                    id: crypto.randomUUID(),
                    content: `${payload.nick} quit${payload.reason ? ` (${payload.reason})` : ""}`,
                    timestamp: Date.now(),
                  },
                ];
              }
            }
          }
          break;
        }
        case "Nick": {
          const server = ircStore.servers.get(payload.server_id);
          if (server) {
            if (server.nickname === payload.old_nick) {
              server.nickname = payload.new_nick;
            }
            for (const channel of server.channels.values()) {
              if (channel.users.has(payload.old_nick)) {
                channel.users.delete(payload.old_nick);
                channel.users.add(payload.new_nick);
                channel.messages = [
                  ...channel.messages,
                  {
                    type: MessageType.SYSTEM,
                    id: crypto.randomUUID(),
                    content: `${payload.old_nick} is now known as ${payload.new_nick}`,
                    timestamp: Date.now(),
                  },
                ];
              }
            }
          }
          break;
        }
        case "Topic": {
          this.ensureChannel(payload.server_id, payload.channel);
          const server = ircStore.servers.get(payload.server_id);
          if (server) {
            const channel = server.channels.get(payload.channel);
            if (channel) {
              channel.topic = payload.topic;
              channel.messages = [
                ...channel.messages,
                {
                  type: MessageType.SYSTEM,
                  id: crypto.randomUUID(),
                  content: `Topic set to: ${payload.topic}`,
                  timestamp: Date.now(),
                },
              ];
            }
          }
          break;
        }
        case "Error": {
          this.addServerMessage(payload.server_id, {
            type: MessageType.SYSTEM,
            id: crypto.randomUUID(),
            content: `Error: ${payload.message}`,
            timestamp: Date.now(),
          });
          break;
        }
      }
    });

    listen<any>("server:status", (event) => {
      const { serverId, status } = event.payload;
      const server = ircStore.servers.get(serverId);
      if (server) {
        server.status = status.toLowerCase() as IrcServerStatus;
      }
    });

    listen<any>("kirc:server_status", (event) => {
      const { serverId, status } = event.payload;
      const server = ircStore.servers.get(serverId);
      if (server) {
        server.status = status.toLowerCase() as IrcServerStatus;
      }
    });

    listen<any>("kirc:server_added", (event) => {
      const payload = event.payload;
      if (ircStore.servers.has(payload.serverId)) return;

      ircStore.servers.set(payload.serverId, {
        id: payload.serverId,
        name: payload.host,
        host: payload.host,
        port: payload.port,
        tls: payload.tls,
        nickname: payload.nickname,
        status: payload.status.toLowerCase() as IrcServerStatus,
        channels: new SvelteMap(),
        serverMessages: [],
      });
    });

    listen<ChannelLockChangedEvent>("channel:lock-changed", (event) => {
      const { serverId, channel, locked } = event.payload;
      this.updateChannelLock(serverId, channel, locked);
    });

    listen<ChannelLockChangedEvent>("kirc:channel_lock_changed", (event) => {
      const { serverId, channel, locked } = event.payload;
      this.updateChannelLock(serverId, channel, locked);
    });
  }

  ensureChannel(serverId: string, channelName: string) {
    const server = ircStore.servers.get(serverId);
    if (!server) return;

    if (!server.channels.has(channelName)) {
      server.channels.set(channelName, {
        name: channelName,
        messages: [],
        users: new SvelteSet(),
        unread: 0,
        locked: false,
      });
    }
  }

  addMessage(serverId: string, channelName: string, message: ChatMessage) {
    const server = ircStore.servers.get(serverId);
    if (!server) return;

    const channel = server.channels.get(channelName);
    if (!channel) return;

    channel.messages = [...channel.messages, message];

    const isCurrent =
      ircStore.currentServerId === serverId && ircStore.currentChannelName === channelName;
    if (!isCurrent && message.type === MessageType.USER) {
      channel.unread += 1;
    }
  }

  removeUnreadMessage(serverId: string, channelName: string) {
    const server = ircStore.servers.get(serverId);
    if (!server) return;

    const channel = server.channels.get(channelName);
    if (!channel) return;

    channel.unread = 0;
  }

  addServerMessage(serverId: string, message: ChatMessage) {
    const server = ircStore.servers.get(serverId);
    if (!server) return;

    server.serverMessages = [...server.serverMessages, message];
  }

  updateChannelLock(serverId: string, channelName: string, locked: boolean) {
    const server = ircStore.servers.get(serverId);
    if (!server) return;

    const channel = server.channels.get(channelName);
    if (!channel) return;

    channel.locked = locked;
  }

  setCurrentServer(serverId: string | null) {
    ircStore.currentServerId = serverId;
  }

  setCurrentChannel(channelName: string | null) {
    ircStore.currentChannelName = channelName;
    if (ircStore.currentServerId && channelName) {
      this.removeUnreadMessage(ircStore.currentServerId, channelName);
    }
  }
}

export const ircService = new IrcService();
