<p align="center">
  <img src="assets/banner.png" alt="velox" width="100%">
</p>

![ci](https://github.com/envykore/velox/actions/workflows/ci.yml/badge.svg)


**fast build time luau optimizer for roblox projects written in rust**

velox runs before ur game loads - it analyzes your rojo project, finds performance problems, dead code and structural issues and then either reports them or fixes them with zero runtime overhead

---

## why?

good question, heres why
roblox games accumulate a lot of weight overtime, i mean a lot. there will be modules that just re-export existing modules, scripts that nothing call, etc. none of these are obvious performance problems and are pretty hard to get rid of without this tooling, unless you do performance updates every other day, which is pretty bad

velox makes it obvious - run it before u sync and it tells u whats wrong and where

---

## what it does

**analyze** (`velox analyze`)
- parses `default.project.json` and walks the entire source tree
- builds a full require graph - stff like scripts dependencies connections like everything
- classifies scripts by type (server, local, module)

finds:
- dead modules - modulescripts that nothing requires (safe to delete)
- circular requires - module a requires module b, module b requires module a
- large scripts - good sign something needs to be split up
- debug noise - counts `print`/`warn` calls and `__DEV__` blocks across all scripts

**build** (`velox build`)
- strips debug calls and `__DEV__` blocks from scripts
- copies the full project to `dist/` - models, assets, project file, everything
- reports calls removed, blocks removed, bytes saved

---
 
## example output
 
```
=== velox ===
 
files
  scripts:      4
  total source: 0 kb
 
dead modules (never required)
  "DeadModule.luau"
 
circular requires
  ModuleA.luau -> ModuleB.luau -> ModuleA.luau
 
debug noise
  5 call(s) across 4 file(s)
  1 __DEV__ block(s)
  run `velox build` to strip
 
3 issue(s) across 4 scripts
```

---

## how this works

velox is a static analysis tool - it reads ur source files directly *without* running them. this means its fast (you wont have time to blink before it analyzes a large project) and doesnt need studio open at all

it understands rojos project structure natively so it knows where your scripts actually live and how they map to the game tree. from there it builds a require graph - every script, every dependency, every connection - and runs analysis passes over it to find problems

the graph uses numeric file ids internally instead of string paths everywhere, so even on large codebases it doesnt slow down a big amount

## install

grab a binary from [releases](../../releases) or build from source:

```sh
cargo install --git https://github.com/envykore/velox
```

## usage

```sh
velox analyze               # find issues
velox analyze --ci          # exits 1 if issues found
velox build                 # strip debug calls and copy to dist/
velox build --out ./output  # custom output dir
```

## config

add a `velox.toml` to your project root:
 
```toml
[analysis]
large_script_threshold_kb = 50
ignore = ["src/Legacy", "src/Vendor"]
 
[build]
strip_debug = true
strip_calls = ["print", "warn"]
```
 
---
 
## roadmap
 
- [x] rojo project parsing
- [x] require graph analysis
- [x] dead module detection
- [x] circular require detection
- [x] large script flagging
- [x] debug call detection
- [x] debug call stripping
- [x] full project build output
- [x] `velox.toml` config
- [ ] constant inlining
- [ ] instance tree analysis (`.rbxmx`)
- [ ] vs code extension
---
 
## license
 
MIT
 