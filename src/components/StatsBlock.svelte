<script lang="ts">
  import { totals } from "../state.svelte";
  import { formatBytes, formatNumber } from "../format";
  import ProgressBar from "./ProgressBar.svelte";
</script>

<div class="mt-3.5 flex flex-nowrap items-center justify-between gap-6 border-t border-border pt-3.5">
  <div class="flex min-w-0 flex-[0_1_auto] items-baseline gap-[5px]">
    <div class="flex w-[120px] min-w-0 flex-none flex-col gap-0.5">
      <strong class="overflow-hidden text-[26px] font-light leading-[1.2] tracking-[-0.5px] [font-variant-numeric:tabular-nums] [text-overflow:ellipsis] whitespace-nowrap">
        {formatBytes(totals().bytes)}
      </strong>
      <span
        class="overflow-hidden text-[10.5px] font-semibold tracking-[0.7px] text-text-faint uppercase [text-overflow:ellipsis] whitespace-nowrap"
      >reclaimable</span>
    </div>
    <div class="flex w-[92px] min-w-0 flex-none flex-col gap-0.5">
      <strong class="overflow-hidden text-[16px] leading-[1.2] [font-variant-numeric:tabular-nums] [text-overflow:ellipsis] whitespace-nowrap">
        {formatNumber(totals().files)}
      </strong>
      <span
        class="overflow-hidden text-[10.5px] font-semibold tracking-[0.7px] text-text-faint uppercase [text-overflow:ellipsis] whitespace-nowrap"
      >files</span>
    </div>
    <div class="flex w-[150px] min-w-0 flex-none flex-col gap-0.5">
      <strong class="overflow-hidden text-[16px] leading-[1.2] [font-variant-numeric:tabular-nums] [text-overflow:ellipsis] whitespace-nowrap">
        {formatNumber(totals().cleanableCategories)} / {formatNumber(totals().totalCategories)}
      </strong>
      <span
        class="overflow-hidden text-[10.5px] font-semibold tracking-[0.7px] text-text-faint uppercase [text-overflow:ellipsis] whitespace-nowrap"
      >categories with data</span>
    </div>
  </div>
  <ProgressBar />
</div>