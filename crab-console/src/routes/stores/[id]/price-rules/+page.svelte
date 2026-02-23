<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Percent, Plus, Pencil, Trash2, X } from 'lucide-svelte';
	import { t } from '$lib/i18n';
	import { authToken, isAuthenticated, clearAuth } from '$lib/auth';
	import {
		getPriceRules,
		createPriceRule,
		updatePriceRule,
		deletePriceRule,
		ApiError,
		type PriceRule,
		type PriceRuleCreate,
		type PriceRuleUpdate
	} from '$lib/api';
	import { formatCurrency } from '$lib/format';

	const storeId = Number(page.params.id);

	let rules = $state<PriceRule[]>([]);
	let loading = $state(true);
	let error = $state('');
	let token = '';
	authToken.subscribe((v) => (token = v ?? ''));

	let showModal = $state(false);
	let editing = $state<PriceRule | null>(null);
	let saving = $state(false);
	let modalError = $state('');

	let form = $state({
		name: '',
		display_name: '',
		receipt_name: '',
		rule_type: 'DISCOUNT' as 'DISCOUNT' | 'SURCHARGE',
		product_scope: 'GLOBAL' as 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT',
		target_id: undefined as number | undefined,
		adjustment_type: 'PERCENTAGE' as 'PERCENTAGE' | 'FIXED_AMOUNT',
		adjustment_value: 0,
		is_stackable: false,
		is_exclusive: false
	});

	let deleting = $state<PriceRule | null>(null);
	let deleteLoading = $state(false);

	function formatAdjustment(rule: PriceRule): string {
		if (rule.adjustment_type === 'PERCENTAGE') return `${rule.adjustment_value}%`;
		return formatCurrency(rule.adjustment_value);
	}

	function openCreate() {
		editing = null;
		form = {
			name: '', display_name: '', receipt_name: '',
			rule_type: 'DISCOUNT', product_scope: 'GLOBAL', target_id: undefined,
			adjustment_type: 'PERCENTAGE', adjustment_value: 0,
			is_stackable: false, is_exclusive: false
		};
		modalError = '';
		showModal = true;
	}

	function openEdit(rule: PriceRule) {
		editing = rule;
		form = {
			name: rule.name,
			display_name: rule.display_name,
			receipt_name: rule.receipt_name,
			rule_type: rule.rule_type,
			product_scope: rule.product_scope,
			target_id: rule.target_id ?? undefined,
			adjustment_type: rule.adjustment_type,
			adjustment_value: rule.adjustment_value,
			is_stackable: rule.is_stackable,
			is_exclusive: rule.is_exclusive
		};
		modalError = '';
		showModal = true;
	}

	async function handleSave() {
		if (!form.name.trim()) return;
		saving = true;
		modalError = '';
		try {
			if (editing) {
				const data: PriceRuleUpdate = {
					name: form.name,
					display_name: form.display_name,
					receipt_name: form.receipt_name,
					rule_type: form.rule_type,
					product_scope: form.product_scope,
					target_id: form.target_id,
					adjustment_type: form.adjustment_type,
					adjustment_value: form.adjustment_value,
					is_stackable: form.is_stackable,
					is_exclusive: form.is_exclusive
				};
				const res = await updatePriceRule(token, storeId, editing.id, data);
				if (!res.success) { modalError = res.error ?? $t('catalog.error'); saving = false; return; }
			} else {
				const data: PriceRuleCreate = {
					name: form.name,
					display_name: form.display_name || form.name,
					receipt_name: form.receipt_name || form.name,
					rule_type: form.rule_type,
					product_scope: form.product_scope,
					target_id: form.target_id,
					adjustment_type: form.adjustment_type,
					adjustment_value: form.adjustment_value,
					is_stackable: form.is_stackable,
					is_exclusive: form.is_exclusive
				};
				const res = await createPriceRule(token, storeId, data);
				if (!res.success) { modalError = res.error ?? $t('catalog.error'); saving = false; return; }
			}
			showModal = false;
			await reload();
		} catch (err) {
			modalError = err instanceof ApiError ? err.message : $t('catalog.error');
		} finally {
			saving = false;
		}
	}

	async function handleDelete() {
		if (!deleting) return;
		deleteLoading = true;
		try {
			const res = await deletePriceRule(token, storeId, deleting.id);
			if (!res.success) { error = res.error ?? $t('catalog.error'); }
			deleting = null;
			await reload();
		} catch (err) {
			error = err instanceof ApiError ? err.message : $t('catalog.error');
			deleting = null;
		} finally {
			deleteLoading = false;
		}
	}

	async function reload() {
		try {
			rules = await getPriceRules(token, storeId);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('catalog.error');
		}
	}

	onMount(async () => {
		let authenticated = false;
		isAuthenticated.subscribe((v) => (authenticated = v))();
		if (!authenticated) { goto('/login'); return; }
		try {
			rules = await getPriceRules(token, storeId);
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) { clearAuth(); goto('/login'); return; }
			error = err instanceof ApiError ? err.message : $t('auth.error_generic');
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>{$t('price_rules.title')} — RedCoral Console</title>
</svelte:head>

<div class="max-w-3xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="font-heading text-lg md:text-xl font-bold text-slate-900">{$t('price_rules.title')}</h1>
		<button onclick={openCreate}
				class="bg-coral-600 text-white px-3 py-1.5 rounded-lg text-sm font-medium hover:bg-coral-700 flex items-center gap-1.5 cursor-pointer">
				<Plus class="w-4 h-4" />
				{$t('price_rules.new')}
			</button>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-20">
				<svg class="animate-spin w-8 h-8 text-coral-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
				</svg>
			</div>
		{:else if error}
			<div class="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
		{:else if rules.length === 0}
			<div class="bg-white rounded-2xl border border-slate-200 p-8 text-center">
				<Percent class="w-10 h-10 text-slate-300 mx-auto mb-3" />
				<p class="text-sm text-slate-500">{$t('price_rules.empty')}</p>
			</div>
		{:else}
			<div class="bg-white rounded-2xl border border-slate-200 overflow-hidden divide-y divide-slate-100">
				{#each rules as rule}
					<div class="flex items-center justify-between px-5 py-3.5">
						<div class="min-w-0 space-y-0.5">
							<div class="flex items-center gap-2">
								<p class="text-sm font-medium {rule.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}">{rule.name}</p>
								<span class="text-[10px] px-1.5 py-0.5 rounded-full {rule.rule_type === 'DISCOUNT'
									? 'bg-green-50 text-green-600'
									: 'bg-orange-50 text-orange-600'}">
									{rule.rule_type === 'DISCOUNT' ? $t('price_rules.discount') : $t('price_rules.surcharge')}
								</span>
							</div>
							<div class="flex items-center gap-2 text-xs text-slate-400">
								<span>{formatAdjustment(rule)}</span>
								<span class="text-slate-300">·</span>
								<span>
									{#if rule.product_scope === 'GLOBAL'}{$t('price_rules.global')}
									{:else if rule.product_scope === 'CATEGORY'}{$t('price_rules.by_category')}
									{:else if rule.product_scope === 'TAG'}{$t('price_rules.by_tag')}
									{:else}{$t('price_rules.by_product')}
									{/if}
								</span>
								{#if !rule.is_active}
									<span class="px-1.5 py-0.5 rounded-full bg-slate-100 text-slate-400">{$t('catalog.inactive')}</span>
								{/if}
							</div>
						</div>
						<div class="flex items-center gap-1 shrink-0 ml-4">
							<button onclick={() => openEdit(rule)} class="p-1.5 text-slate-400 hover:text-slate-600 rounded-lg hover:bg-slate-50 cursor-pointer">
								<Pencil class="w-3.5 h-3.5" />
							</button>
							<button onclick={() => (deleting = rule)} class="p-1.5 text-slate-400 hover:text-red-500 rounded-lg hover:bg-red-50 cursor-pointer">
								<Trash2 class="w-3.5 h-3.5" />
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

<!-- Create/Edit Modal -->
{#if showModal}
	<div class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-50 flex items-start justify-center p-4 overflow-y-auto" onclick={() => (showModal = false)} role="dialog">
		<div class="bg-white rounded-2xl shadow-xl w-full max-w-lg my-8" onclick={(e) => e.stopPropagation()}>
			<div class="flex items-center justify-between px-6 py-4 border-b border-slate-100">
				<h2 class="font-heading font-semibold text-slate-900">{editing ? $t('price_rules.edit') : $t('price_rules.new')}</h2>
				<button onclick={() => (showModal = false)} class="text-slate-400 hover:text-slate-600 cursor-pointer">
					<X class="w-5 h-5" />
				</button>
			</div>
			<div class="p-6 space-y-4 max-h-[70vh] overflow-y-auto">
				<!-- Name fields -->
				<div class="space-y-1">
					<label for="rule-name" class="block text-sm font-medium text-slate-700">{$t('catalog.name')}</label>
					<input type="text" id="rule-name" bind:value={form.name}
						class="w-full rounded-lg border border-slate-300 px-3 py-2 focus:border-coral-500 focus:ring-coral-500 text-sm" />
				</div>
				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-1">
						<label for="rule-display" class="block text-sm font-medium text-slate-700">{$t('price_rules.display_name')}</label>
						<input type="text" id="rule-display" bind:value={form.display_name}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 text-sm" />
					</div>
					<div class="space-y-1">
						<label for="rule-receipt" class="block text-sm font-medium text-slate-700">{$t('price_rules.receipt_name')}</label>
						<input type="text" id="rule-receipt" bind:value={form.receipt_name}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 text-sm" />
					</div>
				</div>

				<!-- Type + Scope -->
				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-1">
						<label for="rule-type" class="block text-sm font-medium text-slate-700">{$t('price_rules.rule_type')}</label>
						<select id="rule-type" bind:value={form.rule_type}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 text-sm">
							<option value="DISCOUNT">{$t('price_rules.discount')}</option>
							<option value="SURCHARGE">{$t('price_rules.surcharge')}</option>
						</select>
					</div>
					<div class="space-y-1">
						<label for="rule-scope" class="block text-sm font-medium text-slate-700">{$t('price_rules.scope')}</label>
						<select id="rule-scope" bind:value={form.product_scope}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 text-sm">
							<option value="GLOBAL">{$t('price_rules.global')}</option>
							<option value="CATEGORY">{$t('price_rules.by_category')}</option>
							<option value="TAG">{$t('price_rules.by_tag')}</option>
							<option value="PRODUCT">{$t('price_rules.by_product')}</option>
						</select>
					</div>
				</div>

				{#if form.product_scope !== 'GLOBAL'}
					<div class="space-y-1">
						<label for="rule-target" class="block text-sm font-medium text-slate-700">Target ID</label>
						<input type="number" id="rule-target" bind:value={form.target_id}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 text-sm" />
					</div>
				{/if}

				<!-- Adjustment -->
				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-1">
						<label for="rule-adj-type" class="block text-sm font-medium text-slate-700">{$t('price_rules.adjustment')}</label>
						<select id="rule-adj-type" bind:value={form.adjustment_type}
							class="w-full rounded-lg border border-slate-300 px-3 py-2 text-sm">
							<option value="PERCENTAGE">{$t('price_rules.percentage')}</option>
							<option value="FIXED_AMOUNT">{$t('price_rules.fixed')}</option>
						</select>
					</div>
					<div class="space-y-1">
						<label for="rule-value" class="block text-sm font-medium text-slate-700">
							{$t('price_rules.value')}
							{form.adjustment_type === 'PERCENTAGE' ? '(%)' : '(€)'}
						</label>
						<input type="number" id="rule-value" bind:value={form.adjustment_value} step="0.01" min="0"
							class="w-full rounded-lg border border-slate-300 px-3 py-2 text-sm" />
					</div>
				</div>

				<!-- Flags -->
				<div class="flex items-center gap-6">
					<label class="flex items-center gap-2 cursor-pointer">
						<input type="checkbox" bind:checked={form.is_stackable} class="rounded border-slate-300 text-coral-500 focus:ring-coral-500" />
						<span class="text-sm text-slate-700">{$t('price_rules.stackable')}</span>
					</label>
					<label class="flex items-center gap-2 cursor-pointer">
						<input type="checkbox" bind:checked={form.is_exclusive} class="rounded border-slate-300 text-coral-500 focus:ring-coral-500" />
						<span class="text-sm text-slate-700">{$t('price_rules.exclusive')}</span>
					</label>
				</div>

				{#if modalError}
					<div class="text-sm text-red-600">{modalError}</div>
				{/if}
			</div>
			<div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-100">
				<button onclick={() => (showModal = false)} class="px-4 py-2 text-sm text-slate-600 hover:text-slate-800 cursor-pointer">{$t('catalog.cancel')}</button>
				<button onclick={handleSave} disabled={saving || !form.name.trim()}
					class="bg-coral-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-coral-700 disabled:opacity-50 cursor-pointer">
					{saving ? $t('catalog.saving') : $t('catalog.save')}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Delete Confirm -->
{#if deleting}
	<div class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-50 flex items-center justify-center p-4" onclick={() => (deleting = null)} role="dialog">
		<div class="bg-white rounded-2xl shadow-xl w-full max-w-sm" onclick={(e) => e.stopPropagation()}>
			<div class="p-6 space-y-3">
				<h3 class="font-heading font-semibold text-slate-900">{$t('catalog.confirm_delete')}</h3>
				<p class="text-sm text-slate-500">{$t('catalog.confirm_delete_desc')}</p>
				<p class="text-sm font-medium text-slate-700">{deleting.name}</p>
			</div>
			<div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-slate-100">
				<button onclick={() => (deleting = null)} class="px-4 py-2 text-sm text-slate-600 hover:text-slate-800 cursor-pointer">{$t('catalog.cancel')}</button>
				<button onclick={handleDelete} disabled={deleteLoading}
					class="bg-red-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-red-700 disabled:opacity-50 cursor-pointer">
					{deleteLoading ? $t('catalog.deleting') : $t('catalog.delete')}
				</button>
			</div>
		</div>
	</div>
{/if}
