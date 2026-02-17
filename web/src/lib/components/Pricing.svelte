<script lang="ts">
	import { reveal } from '$lib/actions/reveal';
</script>

<section id="pricing">
	<div class="container">
		<p class="section-label" use:reveal><span>//</span> PRICING</p>
		<h2 class="section-title" use:reveal>Free to start.<br />Pro when you're ready.</h2>
		<p class="section-desc" use:reveal>
			Your contribution history is yours — never paywalled.
			We monetize depth and convenience, not access.
		</p>

		<div class="pricing-grid">
			<div class="tier" use:reveal>
				<div class="tier-header">
					<span class="tier-tag">[free]</span>
					<h3>Core</h3>
					<div class="tier-price">
						<span class="price">$0</span>
						<span class="period">/ forever</span>
					</div>
					<p class="tier-desc">Everything you need to start contributing.</p>
				</div>
				<ul>
					<li><span class="check">✓</span> Skill profile + onboarding</li>
					<li><span class="check">✓</span> Issue feed (20 results/refresh)</li>
					<li><span class="check">✓</span> 5 AI deep dives / month</li>
					<li><span class="check">✓</span> Contribution tracking</li>
					<li><span class="check">✓</span> Public portfolio page</li>
					<li><span class="check">✓</span> Growth levels</li>
				</ul>
				<a href="#get-started" class="tier-btn">Get Started</a>
			</div>

			<div class="tier tier-pro" use:reveal class:reveal-d1={true}>
				<div class="tier-badge">recommended</div>
				<div class="tier-header">
					<span class="tier-tag">[pro]</span>
					<h3>Pro</h3>
					<div class="tier-price">
						<span class="price">$8</span>
						<span class="period">/ month</span>
					</div>
					<p class="tier-desc">For serious contributors who want the full experience.</p>
				</div>
				<ul>
					<li><span class="check">✓</span> Everything in Core</li>
					<li><span class="check amber">✓</span> Unlimited AI deep dives</li>
					<li><span class="check amber">✓</span> Priority issue matching</li>
					<li><span class="check amber">✓</span> Advanced filters</li>
					<li><span class="check amber">✓</span> Portfolio themes + custom domain</li>
					<li><span class="check amber">✓</span> Skill radar + analytics</li>
					<li><span class="check amber">✓</span> Export data (JSON/CSV)</li>
				</ul>
				<a href="#get-started" class="tier-btn tier-btn-pro">Get Started</a>
			</div>
		</div>
	</div>
</section>

<style>
	section {
		border-top: 1px solid var(--border);
	}

	.pricing-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1.5rem;
		margin-top: 3.5rem;
		max-width: 720px;
	}

	.tier {
		padding: 2rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--bg);
		display: flex;
		flex-direction: column;
		position: relative;
	}

	.tier-pro {
		border-color: rgba(115, 226, 167, 0.3);
		background: linear-gradient(
			135deg,
			rgba(115, 226, 167, 0.03) 0%,
			var(--bg) 50%
		);
	}

	.tier-badge {
		position: absolute;
		top: -1px;
		right: 1.25rem;
		font-size: 0.62rem;
		font-weight: 600;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--bg);
		background: var(--amber);
		padding: 0.25rem 0.75rem;
		border-radius: 0 0 4px 4px;
	}

	.tier-tag {
		font-size: 0.7rem;
		color: var(--text-dim);
		display: block;
		margin-bottom: 0.5rem;
	}

	.tier-pro .tier-tag {
		color: var(--amber);
		opacity: 0.6;
	}

	h3 {
		font-size: 1.15rem;
		font-weight: 700;
		color: var(--text-bright);
		margin-bottom: 0.75rem;
	}

	.tier-price {
		display: flex;
		align-items: baseline;
		gap: 0.35rem;
		margin-bottom: 0.65rem;
	}

	.price {
		font-size: 2rem;
		font-weight: 700;
		color: var(--text-bright);
		letter-spacing: -0.03em;
	}

	.period {
		font-size: 0.75rem;
		color: var(--text-dim);
	}

	.tier-desc {
		font-size: 0.78rem;
		color: var(--text-muted);
		margin-bottom: 1.5rem;
		line-height: 1.6;
	}

	ul {
		list-style: none;
		margin-bottom: 2rem;
		flex-grow: 1;
	}

	li {
		font-size: 0.78rem;
		color: var(--text-muted);
		padding: 0.4rem 0;
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}

	.check {
		color: var(--text-dim);
		font-size: 0.75rem;
		flex-shrink: 0;
	}

	.check.amber {
		color: var(--amber);
	}

	.tier-btn {
		display: block;
		text-align: center;
		padding: 0.6rem;
		font-family: var(--font-mono);
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--text);
		border: 1px solid var(--border-bright);
		border-radius: 4px;
		transition: border-color 0.2s ease, color 0.2s ease, background 0.2s ease;
	}

	.tier-btn:hover {
		border-color: var(--text-dim);
		color: var(--text-bright);
	}

	.tier-btn-pro {
		background: var(--amber);
		color: var(--bg);
		border-color: var(--amber);
	}

	.tier-btn-pro:hover {
		background: var(--amber-bright);
		color: var(--bg);
		border-color: var(--amber-bright);
	}

	@media (max-width: 640px) {
		.pricing-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
