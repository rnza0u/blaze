<img style="display: block; margin-left: auto; margin-right: auto;" src="assets/logos/Blaze-logos-rounded.png" alt="" width="250" />

# Blaze

⚡ A simple and blazing fast monorepo-based build system ⚡

- [Official website](https://blaze-monorepo.dev)
- [Documentation](https://blaze-monorepo.dev/docs/introduction)
- [Pre-build binaries](https://blaze-monorepo.dev/downloads)

## What is a monorepo ?

A monorepo is a git repository where you would store not only a single project or library, but all of your company's code.

The key benefits of using a monorepo with Blaze are :

- **Better code sharing and reusability :** Creating reusable code has never been easier. You don't even need to publish your libraries if they are not intended for public use.
- **Makes teams work in a simpler and more open way :** It can be very difficult to understand how different teams and projects interact with each other when code is splitted into multiple repositories. Having a monorepo allows teams to get a clear understanding of these relations by simply looking at the dependency graph, which is one of Blaze features.
- **Unique CI/CD for your whole company :** Tired of wasting time re-implementing a continuous integration flow for each and every one of your projects ? With Blaze, you can make your whole CI/CD part of your monorepo, and write it once and for all.
- **Allows projects and libraries dependencies to be directly resolved at the workspace level :** The dependency resolution system allows you to describe how each change can impact other parts of your code. Also, when you launch any target, Blaze won't redo what's already done.

## What is Blaze ?

Blaze is a task runner that is designed for monorepos (from small ones to larger ones).

## Blaze features

- Simple and flexible targets configuration
- Support for parallel execution
- Written in Rust
- Fully customizable cache system
- Technology agnostic
- No impact on your application code or runtime overhead
- Easy configuration with JSON, YAML, or Jsonnet
- Community driven plugin system with support for multiple programming languages

## What Blaze is not intended for

- Blaze is not a migration system and it will not care about upgrading your codebase
- Blaze does not perform any code analysis and let you organize project dependencies by yourself
- Blaze really does not care about your code or the technologies that you are using

The main goal of Blaze is to **make development flow faster, simpler and more unified**.