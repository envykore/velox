# velox

**fast build time luau optimizer for roblox projects written in rust**

velox runs before ur game loads - it analyzes your rojo project, finds performance problems, dead code and structural issues and then either reports them or fixes them with zero runtime overhead

---

## why?

good question, heres why
roblox games accumulate a lot of weight overtime, i mean a lot. there will be modules that just re-export existing modules, scripts that nothing call, etc. none of these are obvious performance problems and are pretty hard to get rid of without this tooling, unless you do performance updates every other day, which is pretty bad

velox makes it obvious - run it before u sync and it tells u whats wrong and where

---

## what it does

it analyzes your project 
- parses `default.project.json` and walks the entire source tree
- builts a require graph
- classifies scripts by type

after that it finds problems
- dead modules (modulescripts that nothing requires (safe to delete))
- module a requires module b, module b requires module a
- large scripts (good sign that something needs to be split up into two or three modules)
- useless debug noise
- `__DEV__` guards 

coming soon: optimization on build
- strips debug calls
- inlines constant modules
- removes deadfiles
- reports total size reduction

---

## how this works