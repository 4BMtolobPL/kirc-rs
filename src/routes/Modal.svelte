<script lang="ts">
    import type {Snippet} from "svelte";

    interface Props {
        showModal: boolean,
        header: Snippet,
        children: Snippet
    }

    let {showModal = $bindable(), header, children}: Props = $props();

    let dialog = $state<HTMLDialogElement>();

    $effect(() => {
        if (showModal) {
            dialog?.showModal();
        } else {
            dialog?.close();
        }
    });
</script>

<dialog bind:this={dialog} class="m-auto rounded-md" onclose={() => showModal = false}>
    <div class="w-80 rounded bg-white dark:bg-neutral-800 p-4 shadow-lg">
        {@render header?.()}
        {@render children?.()}
<!--        <button onclick={() => dialog?.close()}>닫기</button>-->
    </div>
</dialog>