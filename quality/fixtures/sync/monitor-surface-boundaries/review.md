# Fixture: monitor-surface-boundaries

## Current verdict — implicit boundary under interaction

Read every Java/Smali subject, the explicit helper/lock model and both complete
outputs on baseline73fc879 and the candidate. The new input retains the original
compiler-cleanup trigger and adds interacting acquisition, handler and outlet
behavior. P1/P3/P4/P5 agree. No input or independent expectation was refreshed.

### Semantics

| Subject | Path-by-path reasoning and judgment |
|---|---|
| nested | Normal order is acquire O,R0,acquire I,R1,release I,R2,sum,release O,return9. With O==I, these remain two acquisitions and two reverse-order releases. R0 failure never acquires I; R1 failure releases I before O and skips R2; R2 failure happens after I release and releases O. Null O enters no body; null I follows R0 and unwinds only O. Agreement for all seven recorded modeled executions. |
| innerCatch | R0 success yields2; ISE failure keeps the real typed catch and its value10. Other errors bypass that typed recovery and undergo the original monitor release before propagation. Only the last private catch-all release/rethrow becomes implicit. Its earlier typed clause and receiving/value copies remain executable. Agreement for all three modeled outcomes. |
| extra | note remains before explicit release on a caught read error. A note failure escapes while the monitor is still held in this deliberately leaking DEX. P6 refuses a natural closed scope, so no automatic release is fabricated. Agreement by source/path reading; this guard is not executed by the fixture model. |
| otherException | Original exceptional release precedes the throw of the supplied parameter; null therefore throws NPE after release. It is not a rethrow of the private capture and retains its explicit outlet/goto/throw. Agreement by source/path reading. |
| twoReturns | Normal paths keep their own release and unlocked return; the mode2 update100 precedes that path's release. A read fault uses its own exceptional release. Multiple normal outlets prevent this bounded implicit surface. Agreement by source/path reading. |
| wrongLock | On a read error, a distinct unowned B release throws IME while A stays held. When A==B it can release A and rethrow E, but absent equality facts cannot assume that case. Original explicit monitors and the lock relationship diagnostic remain. Agreement by source/path reading. |
| escaped | Normal flow returns null; failure returns the exact caught E at the shared join. The captured state is observed by a Phi/shared local outside the handler, so its body assignment remains and no header binding hides it. Agreement for two final-plan model executions. |

The initial challenge exposed typed recovery and compiler cleanup sharing one
Try. The producer now identifies the last catch-all handler index, preserving
earlier real handlers. Every executable compiler-cleanup entrance must already
have natural delivery, and outside rethrow delivery must already be representable.
P7 keeps the original typed clause indices and receiving phases. An annotation
is omitted only when visible catches and the supplied implicit monitor surface
represent all its original clauses; no printed chain points at an invisible
compiler handler.

Six grouped ordinary-Engine tests inspect original events, header binding,
explicit guard events and capture escape. The independent model executed nested,
typed recovery and escape through final selected children/expressions, never
following hidden original CFG destinations to repair a missing final transfer.
Together with the discovery it ran20 scenarios. Extra/changed-error/split-return/
wrong-lock guards remain LLM semantic reading plus product/origin checks, not
executed behavioral observations. Their acquire/release Java notation explains
DEX monitor operations, not added Smali calls.

No full VM/device validation is claimed. The model's supported forms and helper
assumptions are explicit; unsupported forms are failures rather than no-ops.
Within these paths, review found no contradiction and the complexity challenge
supports the repair. The entire prior regression set runs separately for integration.
