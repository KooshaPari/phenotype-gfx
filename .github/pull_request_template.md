> 📋 Read [`CONTRIBUTING.md`](./CONTRIBUTING.md) and [`AGENTS.md`](./AGENTS.md) before opening this PR. Non-trivial features require a linked AgilePlus spec. Touches to the unsafe path (`<AllowUnsafeBlocks>true</AllowUnsafeBlocks>`) must come with a test under `tests/`.

## Summary

<!-- What does this PR do? -->

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Security fix

## Affected surface

- [ ] `GerstnerWaveBank` wave parameters
- [ ] `FluidMesh` procedural mesh
- [ ] `WaterLod` camera-distance tiers
- [ ] Consumer API (sibling project references)
- [ ] Docs / screenshots

## Testing

- [ ] `dotnet build phenotype-water.slnx -c Release`
- [ ] `dotnet test tests/phenotype-water.tests.csproj -c Release`
- [ ] Consumer (e.g. a Phenotype water mod) recompiles cleanly
- [ ] Unity test scene renders without warnings (if applicable)

## Spec / Traceability

<!-- Link the AgilePlus spec, FR IDs, or ADR that this change implements -->
- Spec:
- FR / NFR:

## Risks & Rollback

<!-- Known risks, breaking changes, and how to roll back if needed. Note that public API changes must be backward-compatible; add overloads before deprecating/removing. -->

## Related

<!-- Issues this PR closes; PRs/specs this depends on -->
Closes #
