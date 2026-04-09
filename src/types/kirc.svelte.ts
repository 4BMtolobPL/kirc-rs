import type { SvelteSet } from "svelte/reactivity";

export type ServerId = string;
export type ChannelId = string;
export type MessageId = string;

export enum MessageType {
  USER,
  SYSTEM,
}

export type ChatMessage =
  | {
      type: MessageType.USER;
      id: MessageId;
      nickname: string;
      content: string;
      timestamp: number;
    }
  | {
      type: MessageType.SYSTEM;
      id: MessageId;
      content: string;
      timestamp: number;
    };

export type Channel = {
  serverId: ServerId;
  name: string;
  topic?: string;
  users: SvelteSet<string>;
  unread: number;
  locked: boolean;
  lastReadMessageId?: MessageId;
};

export type Server = {
  id: ServerId;
  name: string;
  host: string;
  port: number;
  tls: boolean;
  nickname: string;
  status: IrcServerStatus;
  serverMessages: ChatMessage[];
};

// TODO: enum으로 바꿀수 있지 않을까?
export type IrcServerStatus = "connecting" | "connected" | "registering" | "disconnected" | "error";
