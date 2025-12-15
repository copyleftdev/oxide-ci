# Environments

## Container (Default)

```yaml
environment:
  type: container
  container:
    image: rust:1.75-alpine
    registry:
      url: ghcr.io
      username: ${{ secrets.GHCR_USER }}
      password_secret: GHCR_TOKEN
```

## Nix

Reproducible builds with Nix flakes:

```yaml
environment:
  type: nix
  nix:
    flake: ".#devShell"
    pure: true
    substituters:
      - https://cache.nixos.org
```

## Firecracker

Isolated microVM execution:

```yaml
environment:
  type: firecracker
  firecracker:
    kernel: oxide/kernel:5.10
    rootfs: oxide/ubuntu:22.04
    vcpu_count: 4
    memory_mb: 8192
```

## Host

Direct execution on agent (requires trust):

```yaml
environment:
  type: host
```
