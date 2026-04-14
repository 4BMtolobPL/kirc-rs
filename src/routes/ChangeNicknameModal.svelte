<script lang="ts">
import Modal from "./Modal.svelte";
import type {ServerId} from "../types/kirc.svelte";
import {invoke} from "@tauri-apps/api/core";

let {serverId, showModal = $bindable()}: {serverId: ServerId | null, showModal: boolean} = $props();

type ChangeNickForm = {
    serverId: ServerId,
    newNick: string,
}

let form = $state<ChangeNickForm>({
    serverId: "",
    newNick: ""
});

const changeNickSubmit = () => {
    if (!serverId) return;

    form.serverId = serverId;

    invoke("change_nickname")
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
                추가
            </button>
        </footer>
    </form>
</Modal>