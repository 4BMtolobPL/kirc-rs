import type { ServerId } from "./kirc.svelte";

export enum ServerStatus {
  Connecting = "Connecting",
  Connected = "Connected",
  Registering = "Registering",
  Disconnected = "Disconnected",
  Disconnecting = "Disconnecting",
  Failed = "Failed",
}

export type ServerDetail = {
  serverId: string;
  host: string;
  port: number;
  tls: boolean;
  nickname: string;
  status: ServerStatus;
};

export type ServerStatusPayload = {
  serverId: string;
  status: ServerStatus;
};

export type UiEventPayload =
  | {
      type: "UserMessage";
      server_id: string;
      channel: string;
      nick: string;
      content: string;
      timestamp: number;
    }
  | { type: "Join"; server_id: string; channel: string; nick: string }
  | { type: "Part"; server_id: string; channel: string; nick: string; reason?: string }
  | { type: "Quit"; server_id: string; nick: string; reason?: string }
  | { type: "Nick"; server_id: string; old_nick: string; new_nick: string }
  | { type: "Topic"; server_id: string; channel: string; topic?: string }
  | { type: "Error"; server_id: string; message: string };

export type ChannelLockChangedEvent = {
  serverId: string;
  channel: string;
  locked: boolean;
};

export type ChangeNickFailedPayload = {
  serverId: ServerId;
  reason: string;
};
