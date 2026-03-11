import type { SvelteMap, SvelteSet } from "svelte/reactivity";

export enum MessageType {
  USER,
  SYSTEM,
}

export type ChatMessage =
  | {
      type: MessageType.USER;
      id: string;
      nickname: string;
      content: string;
      timestamp: number;
    }
  | {
      type: MessageType.SYSTEM;
      id: string;
      content: string;
      timestamp: number;
    };

export type Channel = {
  name: string;
  topic?: string;
  messages: ChatMessage[];
  users: SvelteSet<string>;
  unread: number;
  locked: boolean;
};

export type Server = {
  id: string;
  name: string;
  host: string;
  port: number;
  tls: boolean;
  nickname: string;
  status: IrcServerStatus;

  channels: SvelteMap<string, Channel>;
  serverMessages: ChatMessage[];
};

// TODO: enum으로 바꿀수 있지 않을까?
export type IrcServerStatus = "connecting" | "connected" | "registering" | "disconnected" | "error";
