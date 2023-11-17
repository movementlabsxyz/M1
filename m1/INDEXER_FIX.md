# Indexer Fix
See https://github.com/movemntdev/M1/issues/80.

## Running
@lmonninger has added additional support for running locally via an ANR usage more similar to that used in `timestampvm-rs`. To run locally, you'll need to run the following commands:
```bash
cd m1
./scripts/tests.debug.sh
```

@lmonninger plans to continue to expand this e2e testing suite over the course of the day to provide better automated checks against the indexer and help others debug.

For now, contributors should attempt the continued streaming failure against the indexer when using grpcurl.