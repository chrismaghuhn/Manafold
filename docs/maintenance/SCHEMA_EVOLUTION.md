# Schema Evolution

**Status:** accepted schema-evolution policy  
**Stability:** normative


1. name the exact semantic surface and current version;
2. add/modify reader and writer fixtures before producer code;
3. classify compatibility and migration requirements;
4. update JSON schema, Rust codec, Python DTO, tests, and documentation together;
5. preserve old artifacts and prove reader behavior;
6. never reuse an enum/key value for new meaning;
7. publish migration provenance and content digest.
