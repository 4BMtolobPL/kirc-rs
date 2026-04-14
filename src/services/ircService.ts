import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ircStore } from "../stores/irc.svelte";
import { SvelteSet } from "svelte/reactivity";
import {
  type ChannelId,
  type ChatMessage,
  type IrcServerStatus,
  type MessageId,
  MessageType,
  type ServerId,
} from "../types/kirc.svelte";
import type {ChangeNickFailedPayload, ChannelLockChangedEvent, UiEventPayload} from "../types/payloads.svelte";

export class IrcService {
  async initialize() {
    await invoke("init_servers");
    const initialServers = await invoke<any[]>("get_servers");

    initialServers.forEach((s) => {
      ircStore.servers.set(s.id, {
        ...s,
        serverMessages: [],
      });
    });

    await this.setupEventListeners();
  }

  private async setupEventListeners() {
    await listen<UiEventPayload>("kirc:event", (event) => {
      const payload = event.payload;

      switch (payload.type) {
        case "UserMessage": {
          this.ensureChannel(payload.server_id, payload.channel);
          this.addMessage(
            payload.server_id,
            this.getChannelId(payload.server_id, payload.channel),
            {
              type: MessageType.USER,
              id: crypto.randomUUID(),
              nickname: payload.nick,
              content: payload.content,
              timestamp: payload.timestamp,
            },
          );
          break;
        }
        case "Join": {
          this.ensureChannel(payload.server_id, payload.channel);
          const server = ircStore.servers.get(payload.server_id);
          const channelId = this.getChannelId(payload.server_id, payload.channel);
          const channel = ircStore.channels.get(channelId);
          if (server && channel) {
            channel.users.add(payload.nick);

            if (payload.nick === server.nickname) {
              ircStore.currentServerId = payload.server_id;
              ircStore.currentChannelId = channelId;
            }

            this.addMessage(payload.server_id, channelId, {
              type: MessageType.SYSTEM,
              id: crypto.randomUUID(),
              content: `${payload.nick} joined the channel`,
              timestamp: Date.now(),
            });
          }
          break;
        }
        case "Part": {
          const server = ircStore.servers.get(payload.server_id);
          const channelId = this.getChannelId(payload.server_id, payload.channel);
          if (server) {
            if (payload.nick === server.nickname) {
              // 내가 나간 경우
              if (ircStore.currentChannelId === channelId) {
                ircStore.currentChannelId = null;
              }
              ircStore.channels.delete(channelId);
            } else {
              // 다른사람이 나간 경우
              const channel = ircStore.channels.get(channelId);
              if (channel) {
                channel.users.delete(payload.nick);

                this.addMessage(payload.server_id, channelId, {
                  type: MessageType.SYSTEM,
                  id: crypto.randomUUID(),
                  content: `${payload.nick} left the channel`,
                  timestamp: Date.now(),
                });
              }
            }
          }
          break;
        }
        case "Quit": {
          const server = ircStore.servers.get(payload.server_id);
          if (server) {
            for (const channel of ircStore.channels.values()) {
              if (channel.serverId === payload.server_id && channel.users.has(payload.nick)) {
                channel.users.delete(payload.nick);

                this.addMessage(
                  payload.server_id,
                  this.getChannelId(payload.server_id, channel.name),
                  {
                    type: MessageType.SYSTEM,
                    id: crypto.randomUUID(),
                    content: `${payload.nick} quit${payload.reason ? ` (${payload.reason})` : ""}`,
                    timestamp: Date.now(),
                  },
                );
              }
            }
          }
          break;
        }
        case "Nick": {
          const server = ircStore.servers.get(payload.server_id);
          if (server) {
            // 내 닉네임이 변경된거면 server.nickname 수정하기
            if (server.nickname === payload.old_nick) {
              server.nickname = payload.new_nick;

              ircStore.servers.set(payload.server_id, {
                ...server,
                nickname: payload.new_nick
              });
              ircStore.nickSuccess.set(payload.server_id, payload.new_nick);
            }
            for (const channel of ircStore.channels.values()) {
              if (channel.serverId === payload.server_id && channel.users.has(payload.old_nick)) {
                channel.users.delete(payload.old_nick);
                channel.users.add(payload.new_nick);

                this.addMessage(
                  payload.server_id,
                  this.getChannelId(payload.server_id, channel.name),
                  {
                    type: MessageType.SYSTEM,
                    id: crypto.randomUUID(),
                    content: `${payload.old_nick} is now known as ${payload.new_nick}`,
                    timestamp: Date.now(),
                  },
                );
              }
            }
          }
          break;
        }
        case "Topic": {
          this.ensureChannel(payload.server_id, payload.channel);
          const server = ircStore.servers.get(payload.server_id);
          if (server) {
            const channelId = this.getChannelId(payload.server_id, payload.channel);
            const channel = ircStore.channels.get(channelId);
            if (channel) {
              channel.topic = payload.topic;

              this.addMessage(payload.server_id, channel.name, {
                type: MessageType.SYSTEM,
                id: crypto.randomUUID(),
                content: `Topic set to: ${payload.topic}`,
                timestamp: Date.now(),
              });
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

    await listen<any>("kirc:server_status", (event) => {
      const {serverId, status} = event.payload;
      const server = ircStore.servers.get(serverId);
      if (server) {
        ircStore.servers.set(serverId, {
          ...server,
          status: status.toLowerCase() as IrcServerStatus,
        });
      }
    });

    await listen<any>("kirc:server_added", (event) => {
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
        serverMessages: [],
      });
    });

    await listen<ChannelLockChangedEvent>("kirc:channel_lock_changed", (event) => {
      const { channel, locked } = event.payload;
      this.updateChannelLock(channel, locked);
    });

    await listen<ChangeNickFailedPayload>('kirc:change_nick_failed', (event) => {
      const {serverId, reason} = event.payload;
      ircStore.nickErrors.set(serverId, reason);
    });
  }

  ensureChannel(serverId: ServerId, channelName: string) {
    const server = ircStore.servers.get(serverId);
    if (!server) return;
    let channelId = this.getChannelId(serverId, channelName);

    if (!ircStore.channels.has(channelId)) {
      ircStore.channels.set(channelId, {
        serverId,
        name: channelName,
        users: new SvelteSet(),
        unread: 0,
        locked: false,
      });
    }

    if (!ircStore.messages.has(channelId)) {
      ircStore.messages.set(channelId, []);
    }
  }

  addMessage(serverId: ServerId, channelId: ChannelId, message: ChatMessage) {
    const channel = ircStore.channels.get(channelId);
    if (!channel) return;

    const messages = ircStore.messages.get(channelId);
    if (!messages) return;

    ircStore.messages.set(channelId, [...messages, message]);

    const isCurrent =
      ircStore.currentServerId === serverId && ircStore.currentChannelId === channelId;
    if (!isCurrent && message.type === MessageType.USER) {
      ircStore.channels.set(channelId, { ...channel, unread: (channel.unread += 1) });
    }
  }

  removeUnreadMessage(channelId: ChannelId) {
    const channel = ircStore.channels.get(channelId);
    if (!channel) return;

    ircStore.channels.set(channelId, { ...channel, unread: 0 });
  }

  addServerMessage(serverId: ServerId, message: ChatMessage) {
    const server = ircStore.servers.get(serverId);
    if (!server) return;

    server.serverMessages = [...server.serverMessages, message];
  }

  updateChannelLock(channelId: ChannelId, locked: boolean) {
    const channel = ircStore.channels.get(channelId);
    if (channel) {
      ircStore.channels.set(channelId, {
        ...channel,
        locked: locked,
      });
    }
  }

  updateChannelLastReadMessageId(channelId: ChannelId, lastReadMessageId?: MessageId) {
    const channel = ircStore.channels.get(channelId);
    if (channel) {
      ircStore.channels.set(channelId, {
        ...channel,
        lastReadMessageId: lastReadMessageId,
      });
    }
  }

  setCurrentServer(serverId: ServerId | null) {
    ircStore.currentServerId = serverId;
  }

  setCurrentChannel(channelId: ChannelId | null) {
    ircStore.currentChannelId = channelId;
    if (ircStore.currentServerId && ircStore.currentChannelId) {
      this.removeUnreadMessage(ircStore.currentChannelId);
    }
  }

  getChannelId(serverId: ServerId, channelName: string): ChannelId {
    return `${serverId}:${channelName}`;
  }
}

export const ircService = new IrcService();
