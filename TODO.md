# TODO

I have TODOs scattered across the codebase, but these are the important TODOs that come to mind

## P0
I want to test my service that I wrote. Usually I write the tests before I write the implementation (or at least alongside it). However, because of the time pressure, I just went ahead without them. This does mean decreased confidence in more complex portions like `validate_item` in `menu.rs`. If given another day, this is what I would do first.

- [ ] Unit Tests
- [ ] E2E Tests (natural language prompts to expected orders)

## P1
There are some performance and prompting issues I would like to deep into

- [ ] Performance
  - [ ] **Debug why Tool Calls are not being done in parallel. GPT should be parallelizing the tool calls it makes according [to docs](https://platform.openai.com/docs/assistants/tools/function-calling) but in practice it is not happening often. I think I need to write better prompting to make this happen**
  - [ ] Check performance hit of the `TokioMutex` on the `assistant` object. I think this is low, but I would like to run a profile quickly
  - [ ] Run a quick overall profiler to make sure there are no bottlenecks in the system ([many options for rust](https://nnethercote.github.io/perf-book/profiling.html))
- [ ] Prompting
  - [ ] **Use enums for the function output to make sure that I am getting the correctly formatted options/items**
  - [ ] Upload the menu as a file through the openai platform
  - [ ] Dive deep into whether my system prompt is configured correctly
  - [ ] Check whether my `validate_item` function is prompting correctly
  - [ ] Look into whether my function call descriptions are written correctly
  - [ ] **Investigate adding snippets of the menu as a function call or as part of the prompt to help GPT with recall**
    - [ ] Potentially could also be added as notes from the verifier

## P3
Deployments and Tooling

- [ ] Write nicer build into Cargo
- [ ] Clean up `Cargo.toml`
- [ ] Integrate clippy configurations into the build system
- [ ] Write github actions to run unit/E2E tests
- [ ] Format on Pull Request
- ...

## Intentionally Missing Pieces
- There is no user management, or even API token management
- Only one `menu.json` is supported (although supporting multiple `menu.json` would actually be pretty easy)
- There is no metrics publishing to prometheus/grafana/cloudwatch or anything
- The actual execution of the orders, and sending them to a more permanent place is not done either

