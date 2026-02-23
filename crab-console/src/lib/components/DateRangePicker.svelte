<script lang="ts">
	import { Calendar, ChevronDown } from 'lucide-svelte';
	import { t } from '$lib/i18n';

	type DateRange = { from: number; to: number };
	type Preset = 'today' | 'yesterday' | 'this_week' | 'this_month';

	let {
		onchange
	}: {
		onchange: (range: DateRange) => void;
	} = $props();

	let activePreset = $state<Preset>('today');
	let showCustom = $state(false);
	let customFrom = $state('');
	let customTo = $state('');

	function endOfDay(date: Date): number {
		return new Date(date.getFullYear(), date.getMonth(), date.getDate(), 23, 59, 59, 999).getTime();
	}

	function presetRange(preset: Preset): DateRange {
		const now = new Date();
		const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());

		switch (preset) {
			case 'today':
				return { from: todayStart.getTime(), to: endOfDay(now) };
			case 'yesterday': {
				const yStart = new Date(todayStart);
				yStart.setDate(yStart.getDate() - 1);
				return { from: yStart.getTime(), to: todayStart.getTime() };
			}
			case 'this_week': {
				const day = todayStart.getDay();
				const mondayOffset = day === 0 ? -6 : 1 - day;
				const weekStart = new Date(todayStart);
				weekStart.setDate(weekStart.getDate() + mondayOffset);
				return { from: weekStart.getTime(), to: endOfDay(now) };
			}
			case 'this_month': {
				const monthStart = new Date(now.getFullYear(), now.getMonth(), 1);
				return { from: monthStart.getTime(), to: endOfDay(now) };
			}
		}
	}

	function selectPreset(preset: Preset) {
		activePreset = preset;
		showCustom = false;
		onchange(presetRange(preset));
	}

	function applyCustom() {
		if (!customFrom || !customTo) return;
		const from = new Date(customFrom + 'T00:00:00').getTime();
		const to = new Date(customTo + 'T23:59:59.999').getTime();
		if (from > to) return;
		activePreset = null as unknown as Preset;
		showCustom = false;
		onchange({ from, to });
	}

	function toggleCustom() {
		showCustom = !showCustom;
		if (showCustom && !customFrom) {
			const now = new Date();
			const pad = (n: number) => String(n).padStart(2, '0');
			const todayStr = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
			customFrom = todayStr;
			customTo = todayStr;
		}
	}

	const presets: { key: Preset; label: string }[] = [
		{ key: 'today', label: 'date.today' },
		{ key: 'yesterday', label: 'date.yesterday' },
		{ key: 'this_week', label: 'date.this_week' },
		{ key: 'this_month', label: 'date.this_month' }
	];

	function formatRangeLabel(): string {
		if (activePreset) return $t(presets.find((p) => p.key === activePreset)!.label);
		if (customFrom && customTo) {
			return customFrom === customTo ? customFrom : `${customFrom} — ${customTo}`;
		}
		return $t('date.today');
	}
</script>

<div class="flex flex-wrap items-center gap-2">
	{#each presets as preset}
		<button
			onclick={() => selectPreset(preset.key)}
			class="px-3 py-1.5 text-xs font-medium rounded-lg transition-colors cursor-pointer {activePreset === preset.key
				? 'bg-coral-500 text-white'
				: 'bg-slate-100 text-slate-600 hover:bg-slate-200'}"
		>
			{$t(preset.label)}
		</button>
	{/each}

	<div class="relative">
		<button
			onclick={toggleCustom}
			class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors cursor-pointer {!activePreset
				? 'bg-coral-500 text-white'
				: 'bg-slate-100 text-slate-600 hover:bg-slate-200'}"
		>
			<Calendar class="w-3.5 h-3.5" />
			{#if !activePreset}
				{formatRangeLabel()}
			{:else}
				{$t('date.custom')}
			{/if}
			<ChevronDown class="w-3 h-3" />
		</button>

		{#if showCustom}
			<div class="absolute top-full mt-2 right-0 bg-white rounded-xl border border-slate-200 shadow-lg p-4 z-20 min-w-[280px]">
				<div class="space-y-3">
					<div>
						<label for="dr-from" class="block text-xs font-medium text-slate-500 mb-1">{$t('date.from')}</label>
						<input
							id="dr-from"
							type="date"
							bind:value={customFrom}
							class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-coral-500 focus:border-transparent"
						/>
					</div>
					<div>
						<label for="dr-to" class="block text-xs font-medium text-slate-500 mb-1">{$t('date.to')}</label>
						<input
							id="dr-to"
							type="date"
							bind:value={customTo}
							class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-coral-500 focus:border-transparent"
						/>
					</div>
					<button
						onclick={applyCustom}
						disabled={!customFrom || !customTo}
						class="w-full py-2 bg-coral-500 hover:bg-coral-600 text-white text-sm font-medium rounded-lg transition-colors cursor-pointer disabled:opacity-50"
					>
						{$t('date.apply')}
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>
