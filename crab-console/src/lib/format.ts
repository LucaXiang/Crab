export function formatDate(ms: number): string {
	return new Date(ms).toLocaleDateString();
}

export function formatDateTime(ms: number): string {
	return new Date(ms).toLocaleString();
}

export function formatCurrency(amount: number): string {
	return new Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' }).format(amount);
}

export function timeAgo(ms: number): string {
	const diff = Date.now() - ms;
	const minutes = Math.floor(diff / 60000);
	if (minutes < 1) return '< 1 min';
	if (minutes < 60) return `${minutes} min`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours}h`;
	const days = Math.floor(hours / 24);
	return `${days}d`;
}
