<script lang="ts">
  let { svg, class: cls = '' }: { svg: string; class?: string } = $props();
</script>

<span class="icon-wrap {cls}" aria-hidden="true">{@html svg}</span>
