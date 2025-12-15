# Nix DevShell Example

Pure, reproducible builds with Nix flakes and binary caching.

## Features Demonstrated

- **Flake evaluation** — `nix flake check` validation
- **Pure mode** — No host environment leakage
- **Sandbox** — Isolated builds
- **Binary cache** — Cachix for build artifacts
- **NixOS deploy** — Declarative deployments

## Pipeline Flow

```
check (flake check, format)
       │
       ▼
build (package, devshell, docker)
       │
       ▼
     test
       │
       ▼
cache (push to Cachix)
       │
       ▼
deploy (NixOS)
```

## Flake Structure

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages.default = pkgs.callPackage ./nix/package.nix {};
        
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ rustc cargo clippy ];
        };
        
        devShells.ci = pkgs.mkShell {
          packages = with pkgs; [ rustc cargo nixpkgs-fmt ];
        };
        
        packages.dockerImage = pkgs.dockerTools.buildImage {
          name = "myapp";
          config.Cmd = [ "${self.packages.${system}.default}/bin/myapp" ];
        };
      });
}
```

## Binary Cache

Builds are cached to Cachix for fast CI:

```bash
# Setup (one-time)
cachix use myorg

# Push (in pipeline)
nix build | cachix push myorg
```

## Run Locally

```bash
nix develop              # Enter devshell
nix build                # Build package
nix flake check          # Validate flake
oxide-ci run             # Full pipeline
```

## Why Nix?

| Without Nix | With Nix |
|-------------|----------|
| "Works on my machine" | Reproducible everywhere |
| Version conflicts | Isolated dependencies |
| Docker for everything | Native builds, optional containers |
| CI environment drift | Pinned with flake.lock |
