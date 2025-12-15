# Productization Strategy

## Market Position

### Target Segments

| Segment | Size | Pain Point | Why Oxide |
|---------|------|------------|-----------|
| **Startups** | High volume | GitHub Actions costs explode | Self-hosted, predictable |
| **Security-conscious** | Growing | Shared runners = risk | Firecracker isolation |
| **Nix users** | Niche, vocal | CI doesn't understand Nix | Native flake support |
| **Platform teams** | Enterprise | Need control + compliance | On-prem, audit logs |

### Competitive Landscape

```
                    Hosted ←────────────────→ Self-Hosted
                         │
              ┌──────────┼──────────┐
    Simple    │ GitHub   │ BuildKite│
       ↑      │ Actions  │          │
       │      ├──────────┼──────────┤
       │      │ CircleCI │ Oxide CI │ ← Target quadrant
       ↓      │          │ Drone    │
   Powerful   │ GitLab   │ Jenkins  │
              └──────────┴──────────┘
```

---

## Go-to-Market

### Phase 1: Developer Adoption (Months 1-6)
- Open source core on GitHub
- Hacker News / Reddit / Lobsters launch
- Dev.to / Hashnode technical articles
- Conference talks (RustConf, NixCon)
- YouTube tutorials

### Phase 2: Team Adoption (Months 6-12)
- Free tier with upgrade path
- Team features (OIDC, approvals, notifications)
- Self-serve Stripe billing
- Email nurture sequences

### Phase 3: Enterprise (Months 12+)
- Sales-assisted deals
- SOC 2 compliance
- On-premises deployment
- Professional services

---

## Monetization Levers

### 1. Usage (Primary)
```
Build minutes × Rate = Usage Revenue
```
- Grows with customer success
- Natural expansion revenue
- 70%+ gross margin

### 2. Seats (Secondary)
```
Team size × $15/user = Seat Revenue
```
- Predictable MRR
- Ties to team features

### 3. Enterprise Add-ons
- SSO/SAML: +$5/user
- Audit logs: +$3/user
- Firecracker: +$0.004/min
- Dedicated support: Custom

---

## Metrics to Track

| Metric | Target | Why |
|--------|--------|-----|
| GitHub stars | 5,000 Y1 | Developer interest |
| Weekly active orgs | 500 Y1 | Adoption |
| Paid conversion | 5% | Monetization |
| NDR | 120%+ | Expansion |
| Churn | <5%/mo | Retention |
| NPS | 50+ | Satisfaction |

---

## Open Core Model

### Open Source (Apache 2.0)
- Core scheduler
- Agent runtime
- All execution environments
- Plugin system
- NATS event bus
- Basic API

### Commercial License
- SAML/OIDC SSO
- Audit logging
- Advanced RBAC
- Priority support
- SLA guarantees
- Firecracker images

---

## Content Calendar (Launch)

| Week | Content |
|------|---------|
| -2 | Teaser on Twitter, "Something's building" |
| -1 | GitHub repo public (private star farming) |
| 0 | HN Show: "Oxide CI – Rust-native CI/CD with Nix & Firecracker" |
| 0 | Dev.to deep dive article |
| +1 | YouTube: 10-min demo video |
| +2 | Podcast appearances (Changelog, Rustacean Station) |
| +4 | First case study |

---

## Success Criteria (Year 1)

- [ ] 5,000 GitHub stars
- [ ] 100 paying customers
- [ ] $50K ARR
- [ ] 3 enterprise pilots
- [ ] SOC 2 Type I certification started
- [ ] 2 full-time engineers funded
