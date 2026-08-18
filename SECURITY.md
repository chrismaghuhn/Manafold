# Security Policy

**Status:** accepted pre-release security policy  
**Stability:** process


The project is pre-release and has no stable support window.

Treat malformed card IR, manifests, replay files, trajectory archives,
configuration, and native/Python messages as untrusted. Resource exhaustion,
path traversal, schema confusion, hidden-information disclosure, arbitrary code,
and nondeterministic deserialization are security-relevant.

Before a public repository exists, configure a private reporting contact. Do not
ask reporters to disclose hidden-information leaks or code-execution issues in a
public tracker. Dependencies remain minimal and updates require tests and replay
compatibility review.
