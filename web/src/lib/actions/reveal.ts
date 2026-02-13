export function reveal(node: HTMLElement) {
	const observer = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting) {
					node.classList.add('revealed');
					observer.unobserve(node);
				}
			}
		},
		{ threshold: 0.1, rootMargin: '0px 0px -40px 0px' }
	);

	node.classList.add('reveal');
	observer.observe(node);

	return {
		destroy() {
			observer.disconnect();
		}
	};
}
