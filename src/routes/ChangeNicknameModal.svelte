<script lang="ts">
import Modal from "./Modal.svelte";
import type {ServerId} from "../types/kirc.svelte";
import {invoke} from "@tauri-apps/api/core";
import {ircStore} from "../stores/irc.svelte";

let {serverId, showModal = $bindable()}: {serverId: ServerId | null, showModal: boolean} = $props();
type ChangeNickForm = {
    newNick: string,
}

let form = $state<ChangeNickForm>({
    newNick: ""
});
let error: string | null = $derived.by(() => serverId ? ircStore.nickErrors.get(serverId) ?? null : null);
let successNick: string | null = $derived.by(() => serverId ? ircStore.nickSuccess.get(serverId) ?? null : null);

$effect(() => {
    $inspect.trace("On change nick modal close");
    if (!showModal) {
        if (serverId) {
            ircStore.nickErrors.delete(serverId);
        }
        form.newNick = "";
    }
});

$effect(() => {
    $inspect.trace("On change nick success");
    if (successNick && showModal) {
        showModal = false;
        // showModal을 변경하니까 nickErrors랑 newNick은 위 effect에서 초기화 될듯

        if (serverId) {
            ircStore.nickSuccess.delete(serverId);
        }
    }
});

const changeNickSubmit = () => {
    if (!serverId) return;

    ircStore.nickErrors.delete(serverId);

    invoke("change_nickname", {payload: {serverId: serverId, newNick: form.newNick}});
};
</script>

<Modal bind:showModal>
    {#snippet header()}
        <header class="mb-4 text-lg font-semibold">닉네임 변경</header>
    {/snippet}

    <form class="space-y-3" onsubmit={changeNickSubmit}>
        <input bind:value={form.newNick}
               class="w-full rounded-md border px-3 py-2 dark:bg-neutral-800"
               placeholder="새 닉네임"
        >

        {#if error}
            <p class="text-sm text-red-500">{error}</p>
        {/if}

        <!-- Actions -->
        <footer class="flex justify-end gap-2 pt-4">
            <button
                    class="rounded-md px-3 py-1.5 text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800"
                    onclick={() => showModal = false}
                    type="button"
            >
                취소
            </button>
            <button
                    class="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
                    type="submit"
            >
                변경
            </button>
        </footer>
    </form>
</Modal>