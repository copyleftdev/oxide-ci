# Oxide CI Microsite Specification

## Overview

A single-page marketing site to launch Oxide CI, targeting developers and DevOps engineers.

**URL**: `oxideci.dev` or `oxide.ci`

---

## Page Structure

### 1. Hero Section
```
┌─────────────────────────────────────────────────────┐
│  [Logo]                    [Docs] [GitHub] [Login]  │
├─────────────────────────────────────────────────────┤
│                                                     │
│     Build with confidence.                          │
│     Event-driven CI/CD built in Rust.               │
│                                                     │
│     [Get Started — Free]    [View on GitHub]        │
│                                                     │
│     ┌─────────────────────────────────────┐         │
│     │  $ oxide-ci run                     │         │
│     │  ✓ Build completed in 34s           │         │
│     └─────────────────────────────────────┘         │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### 2. Features Grid
```
┌─────────────┬─────────────┬─────────────┐
│ ⚡ Fast     │ 🔒 Secure   │ 📦 Portable │
│ Rust-native │ Firecracker │ Nix-native  │
│ performance │ VM isolation│ reproducible│
├─────────────┼─────────────┼─────────────┤
│ 🔑 Keyless  │ 📊 Observable│ 🔌 Extensible│
│ OIDC auth   │ OpenTelemetry│ WASM plugins│
│ to clouds   │ tracing     │             │
└─────────────┴─────────────┴─────────────┘
```

### 3. Code Example
```yaml
# .oxide-ci/pipeline.yaml
name: my-app
stages:
  - name: build
    environment:
      type: nix
      nix:
        flake: ".#devShell"
    steps:
      - run: cargo build --release
      - run: cargo test
```

### 4. Pricing Preview
```
┌──────────────┬──────────────┬──────────────┐
│   Starter    │ Professional │  Enterprise  │
│    Free      │  $15/user/mo │   Custom     │
│              │              │              │
│ 500 min/mo   │ 3,000 min/mo │  Unlimited   │
│ 2 concurrent │ 10 concurrent│  Unlimited   │
│              │ OIDC, gates  │  SSO, audit  │
│              │              │              │
│ [Start Free] │ [Upgrade]    │ [Contact Us] │
└──────────────┴──────────────┴──────────────┘
```

### 5. Social Proof (Future)
- GitHub stars counter
- "Trusted by X developers"
- Testimonial quotes

### 6. Footer
```
┌─────────────────────────────────────────────────────┐
│ Oxide CI                                            │
│                                                     │
│ Product: Docs · Pricing · Changelog · Status        │
│ Company: About · Blog · Careers · Contact           │
│ Legal: Privacy · Terms · Security                   │
│                                                     │
│ [GitHub] [Twitter] [Discord]                        │
│                                                     │
│ © 2025 Oxide CI. Apache 2.0 License.                │
└─────────────────────────────────────────────────────┘
```

---

## Technical Stack

| Component | Technology |
|-----------|------------|
| Framework | Next.js 14 or Astro |
| Styling | Tailwind CSS |
| Hosting | Vercel or Netlify |
| Analytics | Plausible (privacy-first) |
| Forms | Formspree or Resend |

---

## Prompt to Generate Microsite

```
Create a modern landing page for "Oxide CI", a developer-focused CI/CD platform.

Design requirements:
- Dark theme with orange (#E85D04) accents
- Hero with animated terminal showing build output
- Feature grid with icons (fast, secure, portable, keyless, observable, extensible)
- YAML code example with syntax highlighting
- Pricing table (Free, Pro $15/user, Enterprise)
- Responsive, mobile-first
- GitHub star button integration

Tech stack: Next.js 14, Tailwind CSS, Framer Motion
Fonts: Inter, JetBrains Mono

Include:
- Navbar with Logo, Docs, GitHub, Login links
- CTA buttons: "Get Started Free" and "View on GitHub"
- Footer with product/company/legal links
- Open Graph meta tags for social sharing
```

---

## Launch Checklist

- [ ] Domain registered (oxideci.dev)
- [ ] Logo finalized (SVG, PNG, favicon)
- [ ] Hero copy approved
- [ ] Pricing page linked to Stripe
- [ ] Docs deployed (docs.oxideci.dev)
- [ ] GitHub repo public with README
- [ ] Twitter/X account created
- [ ] Discord server setup
- [ ] Analytics installed
- [ ] Status page configured (status.oxideci.dev)
