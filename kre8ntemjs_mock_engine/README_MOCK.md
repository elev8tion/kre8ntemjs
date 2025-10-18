# Mock Instrumented Engine

Path: `tools/mock_engine.py`

## Examples
```bash
# Random edges, print only
tools/mock_engine.py --js seeds/example.js

# Write edges to /tmp for file-based scoring
tools/mock_engine.py --js seeds/example.js --write-file

# Crash if input contains the token 'CRASH' or with probability
MOCK_CRASH_RATE=0.1 tools/mock_engine.py seeds/example.js

# Deterministic 'incrementing' style edges based on file metadata
tools/mock_engine.py --mode inc seeds/example.js
```

Exit code: 0 on success; 1 on simulated crash.
