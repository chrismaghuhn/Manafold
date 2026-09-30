# Profiling

**Status:** accepted
**Stability:** process
**Owner:** maintainers

How to find out where the engine spends its time. Profile before changing
code for performance; a guess is not evidence.

## Workload

Profile the random-vs-random smoke game: it plays 30 turns through the
production endpoints, then replays them. Always use a release build, which is
what the smoke gate runs.

For quick per-turn numbers without a profiler, a throwaway release test that
wraps endpoint calls in `std::time::Instant` is enough. Don't commit it.

## Recording with samply in WSL (recommended on Windows)

Native samply on Windows needs the Windows Performance Toolkit (`xperf`, from
the Windows ADK) and an administrator terminal. WSL avoids both.

One-time setup in WSL (Ubuntu):

```bash
cargo install samply --locked
```

Once per WSL start, the owner runs this, because it needs their `sudo`
password. It resets on `wsl --shutdown`.

```bash
echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
```

Build and record from a clone in the Linux home directory, because builds on
`/mnt/c` are slow. Keep the build to 4 jobs: WSL has limited memory.

```bash
git clone /mnt/c/Dev/src/Manafold ~/manafold-perf   # or: git -C ~/manafold-perf fetch
cd ~/manafold-perf && git checkout <commit>
export CARGO_BUILD_JOBS=4 CARGO_PROFILE_RELEASE_DEBUG=line-tables-only
cargo test --release -p mtgml-environment --test random_smoke --locked --no-run
# prints: Executable tests/random_smoke.rs (target/release/deps/random_smoke-<hash>)
samply record --save-only --unstable-presymbolicate -o ~/perf/profile.json.gz -- \
  target/release/deps/random_smoke-<hash> --exact random_games_run_thirty_turns_deterministically_and_replay
```

This writes `~/perf/profile.json.gz` and the symbol file `~/perf/profile.json.syms.json`.

## Reading the profile

Interactive view in the browser:

```bash
samply load ~/perf/profile.json.gz
```

Text summary, which also works for agents:

```bash
python3 scripts/summarize_samply_profile.py ~/perf
python3 scripts/summarize_samply_profile.py ~/perf 'BasicLandEnvironmentRuntimeV8::submit' '^calculate_full_state_digest'
```

The script reads the two files above. It prints:
- functions by inclusive share, where each sample counts a function once;
- functions by self share;
- for each `NAME`: the direct callees of that function;
- for each `^NAME`: its nearest callers.

Names like `fun_ab650` are unnamed libc internals, mostly `memcpy` and `memmove`.

## Driving WSL from Git Bash (agents)

- Put the commands in a script file and run
  `MSYS_NO_PATHCONV=1 wsl.exe -d Ubuntu-24.04 -- bash /mnt/c/<path>/script.sh`.
- `wsl.exe` expands `$VARIABLES` once more before `bash` sees them.
- Without `MSYS_NO_PATHCONV=1`, Git Bash rewrites `/mnt/c/...` into a Windows path.
- Quote `~` (`"~/perf"`) so Git Bash does not expand it to the Windows home directory.
