# Diana pipeline redesign: complete product delivery and stage trust

> Historical design for the retired numbered-stage pipeline. This document does
> not define current architecture, interfaces, refactoring authority or acceptance
> requirements. See the [documentation entry](README.md) for active references.

## 1. Design positioning

This plan defines the target data flow of single-class decompilation: P0 produces
class facts, an independent method reader prepares MethodSource, P1 disassembles,
P2–P7 process the method, P8 prepares and assembles the class, and P9 renders.
Method input preparation adds no P number. The optimisation target is the total
latency of a formal single-class request; `p0_debug` is a fact-inspection entry,
not a performance target.

The execution order and stage contracts described here belong to the historical
design. Its implementation rules and approval language do not grant authority for
current changes. Current contribution rules are in [AGENTS.md](../../AGENTS.md)
and [CONTRIBUTING](CONTRIBUTING.md).

Core principle: **the previous stage delivers a complete product that satisfies its
contract, and the next stage trusts that guarantee and performs only its own
processing. Each stage completes its own algorithm internally, and stages meet
through product contracts rather than depending on another stage's algorithm
implementation or intermediate state.**

"Complete" here has three limits:

- The facts, associations and read states that this stage owes are complete; this
  does not mean that later-stage analysis has finished.
- When the input lacks optional facts, the stage delivers Absent or Unknown
  explicitly; it does not require complete ordinary source metadata.
- Original evidence and analysis results stay distinct; the stage does not have to
  retain all algorithmic scratch data.

Stage types, private construction and tests constrain an implementation together;
they cannot prove that an implementation never has bugs. Information-retention
guarantees come from item-by-item product contracts and transformation
correspondence, not from linear call order alone.

## 2. Invocation, data flow and ownership

```text
application: owns original DEX bytes and the input lease
                       │
                       ▼
shared DEX reader: containers, logical headers, index spaces, safe format reads
                       │
                       ▼
Engine: selects the target class, calls P0
                       │
                       ▼
P0: ClassFacts returns to Engine for the current class request
                       │
             ┌─────────┴───────────────────────────┐
             │                                     │
             │ holds class facts until class       │ enumerates method entries
             │ assembly completes                  ▼
             │                          Engine submits method-level tasks
             │                                     │
             │              ┌──────────────────────┤
             │              │                      │
             │              ▼                      ▼
             │        worker for method A     worker for method B
             │        reader → MethodSource   reader → MethodSource
             │          → P1 → … → P7           → P1 → … → P7
             │              │                      │
             │              └──────────┬───────────┘
             │                         │ owned MethodResult
             └─────────────────────────┤
                                       ▼
                             Engine calls P8 class assembly
                                       │
                                       ▼
                                  ClassResult
                                       │
                                       ▼
                             Engine calls P9 rendering
```

Engine calls P0, the method reader and P1 in order. The data delivery relation is
P0 → Engine → method reader → MethodSource → P1; method declarations and owner
context are retained through MethodSource. The reader belongs to the input-reading
module and is not a new analysis stage.
The scheduler delivers products between stages, and stage implementations MUST NOT
call another stage's analysis-construction entry.

Engine MUST own input association, product ownership, task scheduling and result
joining. Engine MUST NOT parse original declarations, patch missing analysis facts
or execute stage algorithms.

Each worker first prepares MethodSource and then runs P1–P7 in order inside the same
method task;
input preparation and stage completion lead directly to the next call without a
central per-stage queue. Different methods have no class-wide P1, P2 or P3
round barriers.

Required class-level prerequisites are in §8; final class assembly waits for every
started method task to finish.

## 3. Original input and the P0 class product

### 3.1 Shared DEX reader

The shared reader owns containers and logical DEX address spaces, table ranges,
index ranges and encoding syntax. P0, method input preparation, P1 reference
resolution and declaration reads reuse one implementation; the reader does not
produce CFG, SSA or source expressions.

Every reading module MUST check ranges, counts and arithmetic before slicing or
allocating, and MUST retain original physical byte offsets. Same-number indices of
different logical DEX images MUST NOT be merged.

### 3.2 ClassFacts

P0 is the producer of class declarations and member directories, and returns one
immutable `ClassFacts`:

| Part | Facts |
|---|---|
| Class identity and declaration | Original ClassKey, logical header, type ID, descriptor, class flags, superclass and direct interface references |
| Field directory | Original FieldKey, static/instance grouping, declaration, field flags, static ordinal |
| Static preparation | Encoded initial values, original range/tag/bits; explicitly encoded defaults are separate from implicit defaults |
| Method directory | Original MethodKey, direct/virtual grouping, declaration/prototype, method flags, original code offset |
| Metadata | Class/field annotations, optional source file, method/parameter annotation association and location results |
| Read guarantees | Four original group counts, directory coverage, read states and concrete errors of individual declarations and metadata |

P0 MUST NOT open method bodies or decide member hiding, constructor recovery,
generic semantics or source-level nesting ownership. Static encoded initial values
are class-preparation facts; assignments executed in `<clinit>` belong to method
analysis.

A method entry is obtainable only from its owning `ClassFacts`; a caller MUST NOT
compose owner, method and code offset arbitrarily. The scheduler MUST keep the
entry consistent with the original identity of the selected logical DEX.

P0 MUST separate directory-traversal completeness from individual member
declaration failure. A failed field declaration MUST NOT block initial-value reads
that can still be associated safely; a failed interface resolution MUST NOT discard
the original references of the other interfaces.
A suffix that cannot be located safely keeps explicit insufficient coverage
instead of guessing the position of the next record.

### 3.3 Flags and obfuscated input

`ClassAccessFlags`, `FieldAccessFlags` and `MethodAccessFlags` provide the semantic
bit queries each DEX member kind actually needs, while retaining raw and unknown
bits. Consumers MUST use named bits or semantic queries and MUST NOT match whole-word
constants from a sample.

Original grouping comes from class-data and is not inferred backwards from flags.
A synthetic/bridge mark cannot on its own prove that a member may be hidden, and an
unknown bit cannot on its own be a reason to reject a whole class.

Producers MUST retain original names but MUST NOT give name patterns extra
semantics. Source/debug information, Signature, Throws and nested auxiliary metadata
are not prerequisites for starting ordinary method analysis.
Declarations absent from the input stay Unknown; classpath, android.jar or network
facts are not used. Ordinary annotations may have runtime meaning, so an optional
read MUST NOT ignore their original presence or their read problems.

### 3.4 Independent method input preparation: MethodSource

The method reader receives the P0-bound method entry, owner context and the read
capability of the selected logical DEX. Engine calls it inside the method task;
Engine does not implement original record reading itself.

Input preparation owns the code item header, instruction byte ranges, original
try/handler sections and metadata positions, and checks original lengths, counts,
encoding syntax and arithmetic. It does not disassemble the instruction stream,
does not compute a semantic CFG, and does not decide branch or handler targets from
unknown instruction boundaries.

MethodSource retains:

- the original method identity, declaration read result, grouping and semantic flags;
- the necessary owner context and the original logical DEX association;
- mutually exclusive states for present code, confirmed absence of code and read failure;
- original registers/ins/outs counts, instruction bytes and checked ranges;
- original try/handler records, TypeId/descriptor read results of typed catches and their use issues;
- debug/annotation positions and their independent read states;
- available original sections, coverage and concrete local issues.

The presence of a readable code range does not mean the instructions have been
decoded correctly. Reading the original register counts does not mean instruction
operands and parameter entry layout have been checked; those are P1's obligations.
Parsing the encoding of original try/handler records and associating them with
instruction boundaries likewise belong to the reader and P1 respectively.
The reader retains independent read results of typed catches by original type
identity; a failed descriptor read does not erase the already established record
range, target and order, and is not a default reason to stop the whole method.

Input preparation MUST NOT rebuild the class member directory or re-parse method
declarations already delivered by P0.
A section that cannot be located MUST NOT be delivered as a checked slice;
independently readable sections, declarations and metadata keep their fact state.
A member with confirmed absence of code keeps its declaration path and does not
create an empty instruction stream.

Later analysis MUST NOT bypass P1 and use MethodSource to build another complete
disassembly or method analysis. Read-only original presentation may consume checked
sections but cannot claim them as decoded facts.

## 4. Method product model

### 4.1 Maturity and increment

The following is a conceptual structure, not a directly usable Rust signature:

```text
MethodSource   = original method source + safely located sections/records + local read state
DecodedMethod  = MethodSource + original position coverage + instruction/association facts and local availability
FlowMethod     = DecodedMethod + FlowFacts
SsaMethod      = FlowMethod     + SsaFacts
TypedMethod    = SsaMethod      + TypeFacts
StorageMethod  = TypedMethod    + StorageFacts
PlannedMethod  = StorageMethod  + RegionFacts
MethodResult   = P7 method expression + required original facts + concrete result/diagnostics
```

Each stage product MUST express maturity through a different type. Only a
constructor that owns that stage's output guarantee can create the product; the
scheduler cannot wrap a product that lacks analysis.

"Complete" for a mature product means that source coverage, established facts,
local gaps and their impact are accounted for, not that every position was analysed
successfully. A consumer may use the corresponding guarantee only for established
facts, and continues independent work for Unknown or Unavailable under §9.1.
A stage-construction snapshot does not satisfy this guarantee and cannot be
delivered as a mature product after a state label is changed.

"+" denotes ownership transfer and added facts, not cloning all upstream data.
An implementation SHOULD retain the complete upstream product rather than breaking
it apart and moving it field by field with omissions. Local representation
conversions MUST record their source and correspondence.

Stage products MUST NOT express maturity through a catch-all `Option` collection
whose stages may be arbitrarily absent. Declaration-only members and executable
members use different states and cannot fake an empty CFG or SSA.

MethodSource's safe-read guarantee differs from DecodedMethod's
instruction/association guarantee.
P1 MUST retain original provenance and decoded facts even when it cannot finish
disassembly. The scheduler MUST NOT treat safely located bytes or a decoded prefix
as a complete DecodedMethod. Only P1 can publish a DecodedMethod with complete
coverage and local state; its unavailable ranges are not decoded instructions.
That distinction is not a whole-method legality badge and is not a gate on showing
useful original information.
### 4.2 A complete product is not full access

A product owns upstream data, and consumers use the previous stage's formal
semantic interfaces. A product MUST NOT expose all internal stage data and the
entire ProgramIndex through public fields or a `Deref` chain.

The previous stage's interface contains the existing facts that real downstream
consumers need; passing a declared type does not mean P3 must interpret types.
A product cannot retain only the width that the immediately adjacent stage happens
to use and then let P4 re-read original bytes to recover a discarded declaration.

Consumption interfaces MUST distinguish:

- established operation, value, type, storage and protection facts;
- original identities and origins travelling with the product;
- explicit Unknown, Absent, Unavailable and coverage states.

An analysis implementation MUST NOT re-decode original bytes to supply missing
upstream semantic facts. An original view genuinely needed for independent
transform replay is provided by an explicit inspection interface, not by a general
analysis entry.

### 4.3 Stages close internally; products depend one way

"Stages do not depend on each other" means that algorithm implementations and
construction responsibilities do not cross, not that data dependencies are removed.
P3 must consume P2's execution graph, and P4 must consume P3's value sources; without
those products the corresponding processing cannot start. A stage may depend on
formal product interfaces, not on how their producing algorithm is implemented.

Each stage MUST complete its own construction, bounded fact iteration and required
error handling internally. Independent output verification MUST compile only into
tests and is not a processing step of an ordinary method request.
Stage implementations MUST NOT call another stage's algorithm helpers, start its
analysis or patch its results.
Shared identity/coordinate types, resource control and original-format reading stay
as a single shared implementation, but shared facilities must not hide another
stage's whole-method analysis.

The scheduler calls each stage entry and delivers a ready product or a classified
result. Stages do not complete processing through shared mutable method state,
callbacks into another algorithm or cross-stage iteration.
A fixed point belongs inside its stage; P3–P5 are not combined into one mutually
feeding fixed point.

When a consumer needs new facts, it first states the product contract and the actual
producer. The producer completes its own work before delivery, and the consumer must
not add its own parsing or bypass a gap through broad re-exports.
Each algorithm change should stay inside the owning stage; when a contract really
changes, the affected consumers are updated with it.

Independent transformation verification checks this stage's product inside tests and
does not rebuild another stage's analysis product. Its original-evidence obligations
are retained under §6 and do not replay or accept products during ordinary
processing.

### 4.4 Product ownership, construction and consumption interfaces

Product fields and builders stay inside the owning module, consumers receive
immutable queries, and the next stage takes ownership of the upstream product and
retains it inside its own product. The scheduler may move products but cannot take
out mutable internal state, arbitrarily splice products of different methods or wrap
a success type on the owner's behalf.

| Product | Sole producer | Lifetime/ownership | Formal consumers and the queries they need |
|---|---|---|---|
| ClassFacts | P0 class-fact collection | Owned by the current class request | Method reader obtains bound entries; P4 obtains required owner/field/declaration facts; P8 obtains class-assembly facts |
| MethodSource | Method reader | Owned by the method task; borrows DEX sections and P0 declarations | P1 obtains safe code provenance, original counts, declarations, original exception records and independent metadata states |
| DecodedMethod | P1 disassembly and original-association checks | Moved into later method products | P2 obtains instructions, payloads, reference facts, entry layout and original transfer associations |
| FlowMethod | P2 execution-model construction | Held privately inside SsaMethod | P3 obtains ordered reads/writes of operations/terminals, SlotKeys, commit rules, blocks and each transfer kind |
| SsaMethod | P3 SSA construction | Held privately inside TypedMethod | P4 obtains definitions, uses, Phi/incoming, widths, value availability and original operation/declaration constraints |
| TypedMethod | P4 constraints and use interpretations | Held privately inside StorageMethod | P5 obtains representations, use views, incoming adaptations, position-related initialization and required execution relations |
| StorageMethod | P5 storage and transfer construction | Held privately inside PlannedMethod | P6 obtains operations with storage accesses, transfers, copies, snapshots, initialization and lock-value facts |
| PlannedMethod | P6 control planning | Owned during the P7 call | P7 obtains regions/explicit graphs, statement provenance, uses/storage, labels and protection/lock expression plans |
| MethodResult | P7 method expressions | Owned by request collection; then owned by ClassResult | P8 obtains declarations, expressions, original member identities, class-related facts and type uses; P9 renders existing expressions |
| ClassResult / TypeNamePlan | P8 class assembly/name planning | Owned by the report | P9 obtains the complete display product, member outcomes, name plans and origin mappings |

Every stage's formal queries in this table MUST state the availability and gap
impact of the facts concerned, rather than adding local states only to P1–P3 and
letting later stages still cancel a whole product on any single failure.
Concrete responsibilities along the chain are in §9.2.

The upstream facts in the table pass through the semantic interfaces of the directly
preceding product; they do not open the whole ProgramIndex, cross-stage Deref chains
or upstream construction scratch. P6, for example, obtains the execution graph and
storage view already promised by P5 rather than taking P1 and rebuilding the method.
Original inspection evidence and ordinary analysis queries use different read-only
interfaces; inspection cannot become a bypass for analysis construction.

Stage entries use consuming conversion: they consume the preceding product and
return a mature product constructed by this stage's owner or a classified stop
result. Each method task holds exactly one product chain of increasing maturity; it
neither duplicates seven full method fact sets side by side nor creates a global
MethodContext holding every stage's mutable state.

An existing sealed product MUST NOT loosen its guarantee by adding public fields,
public constructors or mutable getters for a migration. New private ownership and
formal read-only queries must stay limited to this table and the needs of actual
consumers; they are not a licence to add arbitrary stage access or to expose all
analysis IR to the application.

### 4.5 Independent read states, stage stops and diagnostics

Original fact results keep the distinction between Present, Absent, NotRead and
Unavailable, and directory coverage separately describes Complete or a locatable
prefix and its termination position. An Unknown interpretation of a fact does not
overwrite a read error, and a complete traversal does not erase a single damaged
declaration. Absence is a fact the owner read out, not a conclusion reached by
turning `Err` into `None`.

Local fact failure and stage-construction stop MUST stay separate. When the owner can
account for source coverage, local gaps and their impact, it MUST publish a product
with explicit local states and let successors continue independent processing; it
cannot escalate every single-item error directly into a whole-method stage stop.

When the product's minimum guarantee or the retention of later results cannot be
maintained, the stage stop retains the method identity, the concrete failure cause,
the last usable product and the stage-local facts that were safely established. A
stop result is not a mature product, and downstream cannot consume it under a mature
contract.
Independent readable content already formed may enter final presentation, but a
missing definition cannot be replaced by a fake ValueId, a default variable or an
empty operation.

The no-code state confirms only that the original code offset is zero; whether it
agrees with the abstract/native bits is a separate original-relationship check. P0
retains the original offset/flags, the reader does not fabricate empty code, and P1
retains the concrete contradiction and available declarations. No-code members do
not need fake Flow/SSA products.

Diagnostics retain the original method, operation/site, slot or edge identity in the
module that produced the problem, and distinguish read failure, unsupported
capability, unknown source/interpretation and internal violation. Engine only joins
original identities and local diagnostic numbers; it does not reclassify or revoke
already confirmed damage from error text.

Ordinary method analysis and local original presentation consume facts already
delivered by the same chain and add no recovery switch, separate ledger or retry
path. The stop scope of resource and InternalInvariant errors follows §9: it cannot
terminate a whole method on kind alone, and it cannot re-run analysis or
successfully truncate output to preserve the last input.
Engine provides no request cancellation callback, token or checkpoint parameter;
the CLI/host owns process-level termination.

## 5. P1–P7 processing and delivery guarantees

### 5.1 P1: disassembly producing DecodedMethod

Input: the MethodSource delivered by the method reader, plus the restricted read
capability that original reference resolution needs. P1 no longer selects the
method, locates the code item or re-establishes the input source itself.

P1 disassembles instruction bytes into a structured instruction directory and
resolves opcodes, widths, register operands, constant bits, references and payloads,
establishing the original association guarantees of branches, results and
exception entries. Disassembly text is data presentation, not a stage protocol, and
promises no Smali or reassembly capability.

A P1 product MUST account for coverage of the original code range: decoded
instructions, payloads, unrecovered fragments with determined ranges, and sections
whose boundaries are undetermined. Instruction format, operands, references and
associations are delivered under their own fact states; one failed reference must
not spread into every fact of an instruction.
Decoded items retain explicit positions, kinds and established operand and
reference facts; an unrecovered section cannot pretend to be instructions. Pure
numeric constants retain bits and width; P1 does not choose the final source type.
Resolved facts cannot merely become "check passed" and then be discarded, forcing
later stages to parse again.

#### 5.1.1 Ownership of input preparation and instruction-association checks

The method reader establishes the safe-read guarantee, and P1 establishes the
structural relation between original instructions and records. P1 trusts
MethodSource's safe ranges and does not re-parse the code item header or already
delivered original records; range and arithmetic checks on the actual read path are
still performed by the shared reader.

| Owner | Check | Protects | Consumers | Failure scope |
|---|---|---|---|---|
| Method reader | ranges, lengths, encoded counts and arithmetic of code/header/instruction bytes/handlers | no out-of-bounds, no overflow, no unbounded allocation from untrusted counts | P1, original-fact consumers | stops unreliable reads and retains checked ranges and usable prefixes; sections it cannot locate safely are not presented as readable |
| Method reader | identity association of method entry, owner and logical DEX | input comes from the selected original method | P1 and method-result association | returns a concrete association error instead of mixing records of different inputs or methods |
| P1 | instruction format, width and complete operands | reliable division of the original instruction stream | P2 instruction-to-operation conversion | stops that range when the width is undetermined instead of guessing boundaries by searching for a later return |
| P1 | register numbers, wide-value spans of instructions, parameter/this entry layout | original instructions refer to register words that exist in the method | P2 operation reads/writes, P3 entry layout obtained through P2 | retains original operands and the problem; does not deliver analysis input that violates the corresponding guarantee |
| Shared reference reader / P1 | original index ranges, reference-declaration reads and the reference category an opcode requires | reliable resolution of original references within the same logical DEX | P2 reference semantics, P4 declaration facts | reader retains read errors and P1 retains instruction-association issues; a missing target definition stays Unknown |
| P1 | branch/switch/handler targets and payload association | targets correspond to determined instruction or data positions | P2 graph construction | retains original targets and association errors and states which associations were not established; no guessing targets or revoking other established facts |
| P1 | invoke argument words, wide-argument adjacency, original producer/result adjacency and category | original call and result layout meets format constraints | P2 argument and result binding | retains original references/words and the failure cause; fabricates no arguments or results |
| Method reader / P1 | original try/handler records and their relation to instruction boundaries | the reader owns record ranges, counts and encoding; P1 owns instruction boundaries, targets and original order constraints | P2 exception-edge construction | readable original records and analysis-usable exception associations stay separate; already read facts are not discarded |
| Method metadata reader / P1 | local reads and association of debug/method-annotation/parameter-annotation records | the reader owns safe record parsing and P1 handles only associations that genuinely need instruction boundaries | P7/P8 metadata consumers | read problems are tracked independently and do not block intact code from entering P2 or revoke an existing input-damage classification |

Items shared by the reader and P1 in this table carry different obligations and do
not require both to verify the same relation twice. Reference-declaration decoding
belongs to the shared reader; P1 associates original instruction references with
the read results.
Safe reading, range and allocation checks cannot be disabled by obfuscated input.

P1's original association checks establish only the fact guarantees consumers need;
they do not prove the ART legality of the whole method. P1 MUST NOT introduce CFG
structuring or source admission under the name of correctness checking.
A legal reference index does not mean the target declaration is complete, and a
producer MUST retain the difference between a missing target and a damaged record.

#### 5.1.2 Obfuscated instructions are not a rejection reason

P1 MUST faithfully disassemble jump chains, control-flow flattening, large switches,
redundant moves and useless or unreachable instructions. As long as the original
format can be read reliably, P1 does not reject a method because of these shapes.

P1 MUST NOT require natural loops, single-entry regions, ordinary constructor
templates, meaningful names or Java-expressible flag combinations. Multi-entry loops
and irreducible graphs are handled by later stages under their own responsibilities;
missing source files, parameter names, local variable names, line numbers, generics
and nested auxiliary metadata do not constitute an admission failure for method
disassembly or ordinary method analysis.

A known encoding with unsupported semantics cannot be disguised as NOP. An unknown
width cannot be guessed. A record that definitely violates the original format stays
Malformed; a known but unsupported format or processing capability stays
Unsupported; when it cannot be determined, the implementation must not claim input
damage merely because the shape looks obfuscated.

#### 5.1.3 Graduated delivery and downstream trust

```text
P0 method entry + selected DEX reader
                    │
                    ▼
method reader: original section location, safe reads
                    │
                    ▼
MethodSource: provenance, readable sections, original counts, local states
                    │
                    ▼
P1: disassembly, original instruction-association checks
                    │
                    ▼
decoded instruction facts + coverage + concrete local issues
                    │
          ┌─────────┴───────────────────────────┐
          │                                    │
source coverage, local states and          the product's minimum
required associations can be accounted for guarantee cannot be maintained
          │                                    │
          ▼                                    ▼
DecodedMethod                          last trusted input and local facts
with established facts and             + classified stop cause
explicit gaps                          entering the read-only display exit
          │
          ▼
P2 processes by fact state without repeating input preparation or disassembly
```

The method reader is the producer of MethodSource, and P1 is the producer of
DecodedMethod. The scheduler delivers these products and does not itself upgrade a
partial result into a complete input through one overall boolean.
Complete instruction coverage and local metadata states may coexist, and an
optional metadata failure must not be merged into a disassembly rejection of the
whole method.

Confirmed absence is Absent, present-but-failed reads are Unavailable, and
deliberate non-reading is NotRead; Unknown means a semantic conclusion cannot yet
be established and is not a synonym for a missing original record.

P2 trusts the guarantees of facts already established in DecodedMethod and consumes
its local states; it does not trust a "fully legal method" badge. A construction
snapshot with a prefix but no suffix coverage and gap states MUST NOT be treated as
a complete method input. A local error does not automatically block P2; only when
the minimum product guarantee cannot be maintained does P2 retain MethodSource and
the local facts and stop the affected construction.

P1 MUST NOT rebuild the class directory, relocate method sources, build a semantic
CFG or SSA, or recover source code.

### 5.2 P2: complete method execution model FlowMethod

Input: a DecodedMethod satisfying P1's coverage and local-state contract. P2 trusts
the established instruction boundaries, original operands and association
guarantees, limits its processing by explicit gaps, and does not disassemble again,
relocate the method source or rebuild original declarations.

P2 answers "which behaviours this method executes, along which paths and in which
order". The design rests on the reconstructed execution-semantics contract, not on
the current P3 data layout or helper functions.
P2's product does not presuppose that later SSA must use fixed entry Phis or a
dominance-frontier scheme.

#### 5.2.1 One product, three correlated fact groups

```text
P1 DecodedMethod
       │
       ▼
P2: instruction semantics, result binding, graph construction and reachability
       │
       ▼
FlowMethod
 ├─ Operations / Terminals: what executes, what it reads/writes, what it effects
 ├─ CFG / Transfers: along which normal or exceptional path it executes
 └─ Origins / Coverage: which original instructions it corresponds to, which exist only as original facts
```

This is not two independent results, an operation list and a CFG. Operations belong
to basic blocks, normal transfers are associated with block-end terminals, and
exceptional transfers are associated with concrete execution positions. P2 MUST
complete these associations internally before delivering the product, and must not
require P3 to patch connections, re-judge instruction semantics or repair P2's
coverage.

FlowMethod retains the required basis of DecodedMethod and adds complete execution
relations. Complete here does not mean that every conditional path is proven
feasible, that every instruction enters the reachable operation graph, or that every
operation and transfer has been recovered. Each execution fact keeps its own
availability; unrecovered behaviour and undetermined transfers retain their
positions, causes and possible impact under §9.1.

#### 5.2.2 Operation contract

Every operation MUST deliver the following facts or their individual unavailable
states explicitly; a missing item must not default to empty:

| Fact | Meaning |
|---|---|
| Identity and origin | method-local operation identity, original instruction identity/position and merged origins |
| Behaviour | original semantics of arithmetic, copies, calls, field/array accesses, allocation, conversions, locks and so on |
| Ordered reads | register slots read, word widths and the required original interpretation |
| Normal writes | destination slots, word widths and when the write happens |
| Commit and exception rules | when a normal result commits, whether an exceptional path commits a register result, and special selection positions |
| Effects | heap read/write, call, may-throw, monitor, class initialization |

These are static execution facts, not values obtained by running the instruction,
and not final Java types or expressions. Original references and formal declarations
come from the P1 product; P2 does not derive external call purity or a complete
inheritance hierarchy.

For example, add-int/2addr is normalised into an integer-add operation reading the
old destination and a source and writing the destination. move, move-object and
move-wide may share copy-processing logic, but P2 MUST retain their widths and
semantic interpretation requirements. Encoded forms may be normalised; original
semantic differences must not disappear.

Terminals are execution behaviour too: their condition, switch selector, return and
throw reads must be delivered explicitly and cannot be omitted because they are not
in the ordinary operation list.

P2 MUST retain original constant bits, field/method/type references and the order of
call argument words. It does not choose source variable types, reject a flow product
that could be established merely because a Java type is not yet expressible, and
does not delete an operation with execution effects because its return value is
unused.

#### 5.2.3 Normal CFG and transfer identity

Basic blocks hold ordered operations and a terminal. The CFG transfer contract MUST
distinguish sequential continuation, original unconditional jumps, condition
true/false, switch case/default and method exit.

Every graph edge has an independent identity and is associated with its source
terminal or exceptional position. Two branches with the same target are not the same
transfer; P2 MUST NOT deduplicate by source/target alone.
Condition and case keys may live in the terminal while edges carry their roles;
the same fact need not be duplicated in both places.

return and uncaught exceptions are exits and do not require invented ordinary basic
blocks to receive them. P2's exceptional transfers are separate from normal
successors, and a handler must not be merged into normal dominance analysis as an
ordinary jump target.

The CFG is the input to later dominance and loop analysis and is not replaced by a
dominator tree. P2's core output does not precompute source regions, nor does it
start an extra dominator-tree or SSA construction for one later algorithm.

#### 5.2.4 Producers/results and exception transfers

P2 binds producers and move-result internally using the original association
constraints established by P1. Merged operations MUST retain every source, normal
output and actual throw position. Adjusting graph-split positions must not rewrite
original try ranges, argument order or instruction positions.

```text
read call inputs → execute call
                  │
        ┌─────────┴────────────┐
        │                      │
   normal completion      throws an exception
        │                      │
        ▼                      ▼
commit destination register   do not commit that destination register
go to the normal successor    select a handler by original priority
```

P2 delivers transfer rules and does not bind concrete old or new ValueIds; P3 adds
value sources. An exception does not mean that heap effects already performed are
rolled back. Special exception-selection positions defined by DEX instructions such
as monitor-exit must be expressed explicitly in the P2 product so that downstream
does not reinterpret the opcode.

Every exceptional transfer MUST retain the throw position, the exception-selection
position, original handler priority, catch types and handler targets. P2 does not
guess which exception a call actually throws to prune paths.
When an exact transfer cannot be established, retain the concrete failure, the known
sources and the potentially affected target ranges; the transfer must not be deleted,
treated as a normal exit or guessed as fallthrough. Other independent facts continue
to be delivered.

#### 5.2.5 Reachability, origin and coverage

P2 computes structural reachability and does not prove satisfiability of conditions.
Flattening, large switches, opaque predicates, jump chains and irreducible graphs are
not on their own rejection reasons for graph construction.
P2 MUST NOT assume conditions are always true, perform branch deletion or reorder
effects for source aesthetics.

The product MUST distinguish the complete original instruction record from the
reachable executed part. Origin/coverage relations state that an original instruction
is carried by an operation or terminal, belongs to a payload, or exists only as an
original fact that did not enter the reachable graph. A merged instruction records
multiple origins; coverage is not proven by equal instruction and operation counts.

Original content outside reachable analysis cannot be cleared or disguised as NOP.
Retaining the original encoding also cannot be treated as having produced that
instruction's complete execution semantics; when required operation semantics are
unsupported the classified result is retained, and later stages may not guess the
behaviour and continue.

#### 5.2.6 Internal processing and output guarantees

P2 completes instruction-semantics conversion and constrained result binding
internally, determines block splits, connects normal and ordered exceptional
transfers, and establishes structural reachability and origin coverage. The concrete
scan and index layout is owned by P2, and its changes must not require consumers to
join internal construction or depend on intermediate construction state.

P2's tests MUST check the correspondence between the input and operation/terminal
reads and writes, commit rules, effects, edge roles and origin coverage. Ordinary
construction delivers the algorithm product directly, does not run that verification,
and does not use P3's SSA construction or P4's type solving to supply this stage's
facts.

FlowMethod's formal interface provides operation inputs/outputs, entry layout,
normal/exceptional transfers and origin facts. Later stages trust these guarantees and
add only their own facts; they do not re-decode instructions, connect the CFG or patch
exception rules. When P2 cannot deliver, it retains identity, original facts and
causes under §9.

### 5.3 P3: exception-aware pruned SSA with on-demand IDF

Chosen algorithm: liveness-pruned SSA, placing Phis with a dominator tree and
on-demand iterated dominance frontiers (IDF), then binding definitions and uses along
the dominator tree. This is the refactoring target that replaces the fixed
block-entry Phi route for ordinary methods; it is not an additional optional
recovery algorithm.

Input: a FlowMethod whose coverage and local availability are fully described,
providing through formal interfaces the established entry layout, ordered reads and
writes, widths, semantic requirements, transfers and commit rules, and the impact
range of gaps. P3 does not depend on P2's builders, scan order or internal
representation-conversion helpers, and does not wait for P4's type conclusions.

P3 produces value sources only: definitions and ValueIds for parameters, this,
operation results and exception objects, plus use bindings and necessary merge Phis.
An SSA definition is a static identity, not a value obtained at runtime.

#### 5.3.1 Why this route

Within the analysis scope where P2 has established control relations, P3 computes
dominance relations and definition merges in one place. The effect of unknown
transfers on entries and availability is handled separately under §9.1; edges cannot
be deleted and the whole-graph guarantee then reapplied. A definition that
dominates the corresponding use binds directly, without creating an entry Phi for
every block of every live-in slot.
Only definitions that can merge enter IDF computation, and unused Phis are pruned
against actual use demand.

This reduces the redundant merges introduced by a fixed entry strategy, and the
target considers both P3 construction cost and the type-propagation, storage, copy
and expression costs it imposes on P4–P7. Being faster than other schemes still
requires measurement on formal single-class requests; fewer Phis do not by
themselves prove optimal total latency or peak memory.

The algorithm follows the dominance, dominance frontier and renaming defined by
[SSA construction in Cytron et al.](https://doi.org/10.1145/115372.115320), combined
with liveness pruning and on-demand IDF. The minimal/pruned property holds only for a
fixed control flow and definition/use model; it does not mean the fewest Java
variables or globally shortest source. DEX exceptions and wide values must establish
correctness separately: a reference paper's proof cannot be inherited to claim that
the whole method transformation is lossless.

#### 5.3.2 P3's internal exception-aware analysis view

P3 uses the precise commit rules already delivered by P2 to build a private
value-state analysis view. Original CFG, operations and exception edges stay
immutable; positions in the view map back to original operations, edges or the
method entry.
It does not re-parse opcodes, infer handlers or add invented execution operations
outward.

```text
P2 call operation
       │
       ▼
P3 private execution position: read inputs, execute call
       │
       ├─ exceptional transfer ────→ handler: keeps old destination values
       │
       └─ normal completion → result commit point → normal successor: obtains new destination values
```

A definition that happens only on normal completion sits at the normal commit
position, not at a position that also dominates the exceptional successor.
P3's SSA dominance analysis MUST include the corresponding normal and exceptional
transfers; computing dominance on the normal CFG alone and then attaching exceptional
inputs to the result is insufficient to establish value availability.
Exception handler priority and special selection positions still come from P2 and
cannot be merged into an ordinary unconditional jump.

The method's initial entry uses a private entry position that provides parameters and
this. Entry back-edges keep independent transfers, and initial values must not be
mixed with later iteration values. A handler exception object is an independent
definition, not an old value of an ordinary register or a call result. An uncaught
exception is an exit and creates no pseudo-handler.

#### 5.3.3 Construction steps

```text
FlowMethod
    │
    ▼
establish private commit/state positions and original associations
    │
    ▼
build definition/use indexes and live demand of affected slots
    │
    ▼
compute the dominator tree of the reachable analysis graph
    │
    ▼
compute IDF over definition sets, placing required Phis with live-in
    │
    ▼
rename along the dominator tree: bind uses, commit definitions, fill edge parameters
    │
    ▼
complete origin availability and state-position mapping
    │
    ▼
SsaMethod = FlowMethod + SsaFacts
```

1. P3 builds definition/use indexes from the real reads, writes and exception commit
   rules of operations and terminals.
   An effect operation with no write produces no pseudo-result, but all of its actual
   inputs are still bound.
2. P3 computes live-in of affected slots backwards with a stage-private work queue,
   processing related positions only when a fact changes. Real required widths and
   overlap relations cannot be reduced to bare register numbers.
3. P3 builds the dominator tree within the private analysis scope where control
   relations suffice to establish the guarantee, without requiring natural loops or a
   reducible shape. Dominator construction uses the Lengauer–Tarjan route with an
   explicit stack; transfers that may enter that scope cannot be ignored.
4. P3 computes liveness-pruned IDF over definition sets of slot states. The
   computation expands by actual definition demand and does not pre-materialise each
   block's complete dominance-frontier set or a register×edge matrix.
5. Once Phi positions are fixed, the corresponding ValueIds are created. P3 maintains
   state versions along the dominator tree, binding all inputs first and then handling
   normal commits. Each incoming is filled from its original transfer identity.
6. P3 computes origin availability and gap impact and retains the Phi closure that
   actual uses need. SsaMethod is sealed only after origin bindings or unavailable
   states and their impact are fully accounted for; independent correspondence checks
   run only in tests.

A fixed definition set and graph may require bounded iteration, but ValueIds are not
created or replaced in a P3/P4 feedback loop. Different control transfers that point
to the same block still carry separate Phi incomings.
A use binding may reference a definition that dominates the use directly across
blocks; passing through an entry Phi is no longer required.

P3 performs no constant propagation, operation deletion or source-oriented variable
merging. Provably redundant or useless Phi handling belongs to SSA-internal
normalisation and must retain original operation identities and required semantic
requirements; Phi deletion must not expand into deleting calls, casts, heap accesses
or exception paths.

#### 5.3.4 Wide values, overlapping coverage and undefined states

Standard IDF depends on an explicit variable-state model. DEX register words are
overwritten in crossing patterns by single-word and wide writes, so two words cannot
be renamed independently and then arbitrarily assembled into one wide ValueId.

P3 uses fixed SlotKey=(start register, One|Two) whose word interval is
[start,start+width). The slot set arises only from parameters, reads and writes
already delivered by P2; it does not guess new slots during IDF or type solving and
does not pre-build a register×edge state matrix.

Each SlotKey is an independent state variable in the private analysis graph. Its state
sources are:

- Value: an original definition of the same start and width, or an established Phi value;
- Undefined: the slot has no corresponding parameter or definition at the original method entry;
- Invalidated: an overlapping write with original provenance means the slot no longer carries a complete value;
- origin not establishable: a write, commit or entry relation that P2 states as a gap
  affects the slot, retaining the concrete position, impact range and cause; this is
  not conflated with Undefined, Invalidated or construction Pending.

Invalidated and Undefined are state definitions, not executable default values.
Construction-time Pending is separate from these known states, and Pending must not be
treated as Undefined before all connections are made.
A successful value binding must preserve the exact slot; a single-word read cannot
guess a value from the high word of a wide value, and a wide read cannot splice two
independent single-word definitions.

A write to destination D establishes the following transfer at its actual normal
commit position:

```text
all inputs are read from the state before the write

state[D] = Value(this result)
for every tracked slot S whose word interval overlaps D but whose SlotKey differs:
    state[S] = Invalidated(this original write)

states of slots that do not overlap are unchanged
```

Entry parameters provide initial values at their exact slot, and differently sized
slots that overlap them do not obtain a readable value.
An implementation may use a sparse register-to-slot index to look up overlap
candidates; widths are only One/Two, so a full slot scan per write is unnecessary.
Every index and interval computation still checks ranges and platform capacity.

The definition-position set MUST include overlapping invalidation, not only explicit
value writes to that slot.
Invalidation arises only on normal commit; an exceptional operation commits neither a
result nor the invalidation caused by this normal write. When there are several
outputs, all inputs are read first and states are then established under P2's overall
commit rules.

Liveness and on-demand IDF work over these complete state variables. During renaming
along the dominator tree, the version stack handles values, Undefined, Invalidated and
origin-not-establishable states together; Phi incomings retain their exact edge origin
or the explicit impact of an undetermined entry.
A state Phi's result may become a complete value, an invalidation or unproven
availability; the presence of one valued input must not cause other inputs to be
ignored by default. After state propagation, SsaFacts delivers value widths and the
availability result of each use, and P4 no longer pairs register words.

```text
initial definition: write (v0,Two) = x1
    state[(v0,Two)] = Value(x1)

later write: write (v1,One) = x2
    state[(v1,One)] = Value(x2)
    state[(v0,Two)] = Invalidated(this write)

read (v0,Two) after the two paths merge:
    one incoming = Value(x1)
    the other incoming = Invalidated
    x1 alone cannot be used to claim a complete wide value
```

This model does not modify original registers, CFG or operations, and does not split
an original wide value into two ordinary ValueIds that may be recombined arbitrarily.
Standard SSA placement and renaming apply over this state model; minimality holds
only relative to that state graph and demand, and does not directly claim the fewest
original operation Phis or source variables.

P3 MUST NOT fill an invalid state or missing definition with zero, null or an
arbitrary value. Whether a current use definitely violates input constraints is
separate from being unable to prove availability because of a conservative path set;
without path-feasibility evidence, all potential bad sources must not be declared
already-executed Malformed.
Construction omissions, wrong Phi connections and foreign identities remain
InternalInvariant.

#### 5.3.5 Output and test verification

SsaFacts MUST deliver definitions, use bindings of all operations/terminals, normal
results, required Phis, per-edge incoming associations, and concrete handling results
for value width and availability.
The original execution model and declaration/semantic requirements are retained
through SsaMethod, and P4 does not re-guess register sources.

P3 tests check that a definition is available at the execution position of the
corresponding use or incoming, that intra-block reads respect read-before-write, that
normal and exceptional origins agree with P2's commit rules, and that wide-value
ownership is complete.
Dominance is one basis of value availability and does not replace wide-value and
exception-commit checks.

Test verification must not trust builder counts or booleans alone. Existing
independent original-replay test obligations keep explicit ownership during
migration under §6; P3 does not call P2's graph-construction algorithm or re-check
original call encodings.
On completion, liveness, version stacks, IDF and private construction scratch are
released, while the products and correspondences later stages need are retained.
Standard IDF's local complexity must not be claimed as linear cost for SSA of a whole
DEX.

#### 5.3.6 Exceptional commit graph and output position mapping

P3's private analysis graph is built from FlowMethod's known blocks, operation order
and transfers and does not read original opcodes. Each parameter initial definition,
operation input read, normal commit and handler exception-object binding has an
explicit position, all mapped back to original method-entry, operation or transfer
identities.

For a result-producing operation that may throw, the exceptional edge leaves the
execution position while the normal path passes the commit position to obtain the
result and slot invalidation; a result-less operation creates no pseudo-value
definition. A throwing terminal has no normal commit, only the original exception
chain or an outward exit. The special rules of enter/exit consume P2 directly.

The private graph exists to establish value availability, not to rewrite original
control flow. P3's tests must check that commits on original normal paths, old states
on exceptional paths and handler bindings agree with the view. Using one uniform
environment at the end of the source block for every exceptional edge is forbidden.

A Phi may sit at a private state position, so SsaFacts MUST deliver its StatePoint
identity and original entering-transfer mapping. Downstream obtains the Phi's
entry/execution obligations through the preceding product and must not assume that
each Phi belongs to an original block head or that each ValueId's availability is
determined only by numeric PC order.
Necessary synthetic positions carry only states and copies and generate no new calls,
heap effects or runtime dispatch; P5 realises the corresponding transfers at original
normal edges, commit successors or handler entries.

Output-position mapping must be completed during P3 construction; P5 does not call
P3 helpers to derive which original edge corresponds to some private predecessor.
Discovering a missing position later is a product violation, and P6/P7 must not guess
it.

### 5.4 P4: value facts and use interpretations producing TypedMethod

Input: value sources already delivered in SsaMethod, original operation requirements,
reference-declaration facts and the current class declaration.
The current class declaration travels with the method product as an immutable handle
from P0, and P4 does not fetch context from the scheduler separately.
Ordinary reference reads require no inheritance proof; cross-class declarations that
behaviour-relevant consumers genuinely need still follow the restricted read-only
query contract of §7.2, and the complete hierarchy is not queried for Java assignment
or overload legality.

P4 collects declaration constraints, solves value families/types, Phi joins, constant
use views and conversions at each use, and produces initialization, array and
call-related evidence. It does not modify the graph or reallocate ValueIds.
P2's execution-fact interface delivers original catch-type read results and the array
payload width, count and borrowed bits associated with an operation. P4 consumes these
established facts and does not re-parse from the original code view.
The original definition of an exception object and each typed catch declaration are
retained; a shared handler retains every possible declaration, and no separate
Throwable-assignability propagation domain is built for ordinary reference reads.
Array-fill element interpretation comes from the SSA array component; int/float cannot
be chosen from payload width alone, and a bulk fill must not become new stores or
change exception positions.

#### 5.4.1 Consuming original reference signatures directly

A reference record in DEX is itself a usable fact and does not require the target class
definition or method implementation to exist in the analysis input. P4 MUST consume
the already delivered owner, name, prototype, invoke kind and parameter/return
descriptors directly and must not re-parse the reference record to obtain the same
facts.

For example, the original reference `Ljava/lang/String;->length()I` already supplies
the owner, method name, no explicit parameters and an int return constraint. Even when
the input contains no String class definition, P4 still uses these facts.
The receiver, argument order and call form come from existing execution products and
cannot be changed into an unknown method or stripped of the original reference
identity because a definition is missing.

A reference signature does not prove the concrete implementation chosen at runtime,
nor does it provide a complete inheritance relation, definition flags, overload sets or
Throws metadata. P4 keeps Unknown for facts the input does not provide; a query must
not turn a missing definition into an entirely unusable reference, nor depend on
classpath or external libraries to fill it in.
Missing source-binding information cannot independently become a method output gate.
An ordinary input whose origin, width and reference read are established is delivered
as Direct(Reference) rather than as a read failure because the inheritance relation is
unknown.
This does not prove subtype, Java assignment compatibility or prior initialization of
the object; original declarations are still retained separately.
A call's return type comes from the original reference or an independent invocation
prototype and is not re-selected from argument types.
When a value interpretation or conversion that preserves original semantics genuinely
cannot be established, P4 retains the concrete problem and fabricates neither call
target nor conversion.

#### 5.4.2 ValueFacts and UseViews are separate

P4 MUST separate facts about a value from the interpretation at each use, and must not
express original bits, value types, use requirements and conversion results through one
type field that is rewritten repeatedly.

| Product part | Delivered content |
|---|---|
| ValueFacts | ValueId, original producer-fact association, width, family/concrete-type facts, and evidenced zero-value origins and ranges |
| UseViews | use position of an operation or terminal, corresponding ValueId, original semantic/descriptor requirements, and the interpretation or conversion result |
| PhiViews | the Phi's common representation, the value and adaptation result of each incoming, associated by original edge identity |

ValueFacts associates original bits and producing operations in existing products and
does not copy a second original record.
UseViews MUST cover every retained operation and terminal input, stating direct reads,
literal views, evidenced conversions, unknown or concrete unrepresentable reasons, and
must not leave a bare None for later stages to guess.

```text
ValueFacts[x1]: original 32-bit constant, bits=0
                          │
                ┌─────────┴──────────┐
                ▼                    ▼
Object parameter use of call A   int parameter use of call B
UseViews[A, parameter 0]         UseViews[B, parameter 0]
    → NullLiteral                     → IntLiteral(0)
```

These use views neither rewrite original bits nor insert a runtime integer-to-object
conversion. P4 does not turn all other uses into references because of one reference
use, nor create a whole-method type conflict because of one integer use.

An ordinary runtime int value cannot obtain a null view because it may be zero at one
use. Numeric conversion differs from bits interpretation: a runtime int-to-float
conversion cannot replace the floating-point interpretation of an original constant.
Only original conversion semantics or explicit value-preserving evidence may produce
the corresponding conversion.
A zero-value view propagated through move/Phi needs established origin facts; it
cannot be admitted merely from a register number or the appearance of zero somewhere,
and a merge of nonliteral values still needs an explicit common representation.

Equal-width Phi inputs are not compatible by themselves. P4 MUST deliver the common
representation, incoming adaptations and ordinary use interpretations separately. P4
MUST first establish an as-is read with available origin, matching width and compatible
representation, and only then choose an evidenced value-preserving view. A use
requirement does not prove a value's range in reverse; an original declaration does not
automatically produce a lossless narrowing request. Insufficient range evidence may
block only an optional view, not deny an established read; literals and runtime values
follow the same rule. Downstream cannot turn an ordinary integer expression into a
boolean expression from a Z declaration alone.
The original int-to-byte conversion and storage truncation still execute at their
original operation positions and gain no extra conversion for display purposes.

#### 5.4.3 Downstream delivery and the stage's own checks

TypedMethod retains SsaMethod and encapsulates ValueFacts, UseViews, PhiViews, original
declaration constraints, position-related initialization facts and concrete
uncertainties.
TypeFacts MUST provide a handling result for every retained value and use: an
established fact, explicit unknown, or the concrete reason preventing that
representation. Unknown cannot be disguised as a proven conversion.
P4 MUST propagate problems along the real constraints of the values, uses and
incomings concerned and keep solving unaffected parts; an empty candidate set or a
single failed conversion must not be the default whole-method error behavior.

P5 consumes the representations and incoming adaptations already determined by P4 to
schedule storage and edge copies and does not re-select types. P7 consumes UseViews to
express operations and conversions, does not guess whether it should print 0, null or a
floating literal, and adds no arbitrary cast, boxing or helper for source convenience.

Declaration binding, type solving and final source-use conversion checks carry
different obligations and cannot be merged into one "all correct" boolean flag. P4
tests check the types and conversions it establishes and do not redo SSA.

#### 5.4.4 Initialization facts do not presuppose a natural new expression

P4 establishes initialization facts from existing allocations, constructor calls, SSA
origins and execution transfers: the original allocation identity, constructor
reference, receiver/arguments, aliases and the initialization state at each use.
A known constructor signature is consumed directly, for example
`Lexample/T;-><init>(I)V`, without requiring T's complete definition or constructor
body to exist in the input.

Normal completion of an allocation produces an uninitialized reference; the
constructor call acts on the same object, allocates nothing further and returns no
replacement object. Initialization state is associated with execution positions and
paths, and reference availability after normal completion must not propagate to a
constructor-failure path. Heap/call effects the constructor already performed are not
rolled back by an exception, so P4 cannot interpret initialization failure as the object
and its effects never having existed.

P4 MUST determine initialization state from existing SSA reference bindings and
execution positions and must not make unique object identity an admission condition.
A Phi uses the post-transfer state of each incoming; a merge whose sources are all
initialized stays initialized even when the identity is Multiple. Paired references
select their source along the same incoming, and equal source sets must not be treated
as the same object. Copy/Cast capture a reference without freezing initialization state;
later constructor events still act on the proven associated object. A static allocation
position does not mean the same dynamic instance inside a loop. Unavailable origins and
unrooted loops cannot supply completion evidence.

A constructor-this awaiting initialization may consume a proven indirect ancestor
relation within the input, but P4 MUST deliver it separately from a direct superclass
relation; an original ancestor constructor call must not be rewritten as an ordinary
`super()`. Owner matching of a newly allocated object still uses the original allocation
type and does not accept an arbitrary ancestor because of this. A missing superclass
definition cannot serve as owner-mismatch evidence; the current query is limited to the
same logical DEX.

P4 MUST separate original initialization relations from the conditions for merging
them into a natural `new T(...)`. Being unable to prove a natural merge does not mean
the original allocation/initialization facts are missing, and known execution facts must
not be withdrawn merely because of a block boundary, an intervening operation or a
different protection chain. Known relations go to P5/P7, which support natural or
explicit expression through the corresponding product; unestablished origins, states or
legality keep concrete Unknown or classified problems. A clear relation between an
ordinary allocation and a constructor call is not an expression problem merely because
DEX splits it into two instructions.

P4 does not modify ValueIds, add writes or require a fixed compiler template in order to
satisfy Java constructor syntax. The method product concerned MUST retain every
independent known operation and must not let a restricted natural-expression profile act
as analysis admission for the whole method. Confirmed input damage and internal
violations keep their original classification.

#### 5.4.5 Separated constraint domains and sparse solving

P4's constraint sources are fixed to declarations, operation requirements, definitions
and use relations already delivered by preceding products.
It does not modify SSA identities, nor does it present a type the consumers desire as a
fact already known to producers. Internal solving uses distinct domains, and one mutable
Kind does not perform narrowing, joining and Unknown at once:

| Domain | Initial facts and relations | One-way change |
|---|---|---|
| Representation family | word widths, original opcode semantic requirements and descriptors; compatibility relations of nonliteral values | merge equivalence groups; the group's Allowed family set is intersected, and excluded candidates are never re-admitted |
| Type/array origin | declaration seeds such as parameters, fields, call returns and new-array; move/Phi and bounded component projections | known origins only accumulate, Unknown is retained independently, and array rank decreases only along actual component accesses |
| Positional reference type bound | receiver and target declaration of an original check-cast plus its normal transfers; original SSA copies, Phi incomings and load positions | refine missing declaration bounds on covered normal paths without modifying source-value facts or repairing unusable input; bounded position/projection queries form the closure |
| Zero-value origin | original zero constants, value-preserving moves and Phi incomings | establish a bounded closure whose sources are all zero; unrooted loops cannot supply zero evidence |
| Numeric range | fixed ranges of original constants and declared/narrowing operations | propagate through move, hull of Phi intervals; bounds come from fixed seeds and are never derived backwards from use requirements |
| Reference identity origin | parameters, allocations and other reference producers; moves, original casts and Phis | None→One(root)→Many; the same family does not mean the same root |
| Initialization state | allocation identity, this, original constructor calls and normal/exception commits | propagate state and identity by execution position, keep per-path differences on merges, and do not globally turn Uninit into Ref |

Family equivalence constraints use a union-find, but merge only representations that must
be compatible, never ValueIds, object identities or exact reference types. A pure
constant establishes an independent view node per use position, and a reference use and
an integral use of the same zero constant are not forced into one runtime family.
Available literal inputs of a runtime Phi MUST still constrain that Phi's common
representation: compute admissible families from original bits and width and collect
them as local Phi constraints before nonliteral equivalence groups are merged.
A nonzero single word cannot supply a reference representation; zero and wide values
still keep their own evidenced candidates. This constraint neither merges literal source
nodes nor changes their interpretation at other uses. A conflict must not pollute
independent producers, unusable input cannot supply literal evidence, and a pure literal
Phi whose bits are all equal still keeps per-use views.

Array components are delivered by P4's unified declaration projection and positional
type-bound solving, retaining known component sets and independent Unknown; alternative
path merges take the union of known origins and keep every uncovered gap. A non-array
declaration upper bound does not prove that the runtime object is not an array, and a
known branch cannot cover an unknown branch. An original check-cast has no SSA output;
P4 MUST apply the checked bound only after its normal completion for a proven identical
reference, and must not rewrite the original source declaration, backfill positions
before the check, pollute exceptional paths or fabricate writes. A Phi selects its
reference along the original incoming transfer, multi-level array projections are
queried at the original load position, and an array and its element are not aliases.
A successful null check contributes no component type from non-null origins and does not
prove non-nullness or that the original array access executed normally.

ArrayAccessView delivers the same positional facts for the original get/put/fill, and
downstream does not project again. Ordinary reads and writes depend on the original
opcode encoding and SSA bindings and are not gated on complete component declarations;
the payload's element interpretation still requires sufficient component evidence.
A complete positional bound also does not authorize deleting the original null, bounds
or covariant-store checks.

Type origins and array components first form a usable constraint closure, and family
selection ends afterwards; aget, for example, cannot default to int first and then
return upstream for a rerun because a float[] origin appeared. Dependencies between
domains are realised by bounded events and work queues inside P4 and do not depend on
P5/P7 expression attempts.

The solving index includes reverse relations from ValueId to operation uses, Phi uses and
related constraints. Only fact changes notify affected uses; each round does not
re-decode instructions, re-read the whole declaration directory or scan all unrelated
operations. Delivered original signatures become constraints directly; a newly added
in-input relation query keeps its original classified errors, coverage and Unknown and
does not mask read failures with an empty candidate set.

Source-type facts stay separate from the common representation used for storage and
display. A known source descriptor must not be rewritten to Object merely because a join
chose Object, and an unknown hierarchy must not be claimed as a proven subtype.
Only original effect-free interpretations or established safe conversions may generate
the corresponding UseView/PhiView; unknown static binding may accompany the original
call for display and must not be solved with a new runtime cast.
A pure numeric Phi transfer copies only same-width word/wide encodings; when no numeric
interpretation is required, Runtime's numeric candidate set explicitly denotes common
bits storage, and incomings retain CopyBits/LiteralBits.
P5 consumes that already selected storage and does not choose int/float or long/double
again. Mixed reference and numeric candidates still must establish a compatible
representation; an ordinary numeric read also keeps its own interpretation requirement.

The change direction, seeds and propagation relations of every domain are fixed inside
P4. The state space comes from the current method and the input facts actually queried,
and finiteness is not proven by a default quota or solver timeout. Results are delivered
once after representation completes: P5 does not re-select a family, and P7 does not feed
back whether Java generation succeeded.

### 5.5 P5: per-edge out-of-SSA and storage transfer plans

Chosen algorithm: type-explicit, per-edge SSA elimination (out-of-SSA), ordering parallel
copies through a copy-dependency index and a ready queue, breaking cycles with temporary
storage, and realising required handler value transfer through exception snapshots.
Independent storage is established first, after which P5 performs constrained storage
reuse (coalescing) over original Move/copy relations. Reuse must satisfy
exception-aware liveness, interference and representation compatibility conditions; it
cannot depend on P6's syntax candidates or change original commit and copy positions.

Input: definition/use relations of TypedMethod, ValueFacts, UseViews, PhiViews and
position-related initialization facts. P5 trusts existing origin, width, type and
adaptation guarantees, does not rebuild SSA and does not re-derive types or decode
original instructions.

#### 5.5.1 Three related jobs, one product

```text
P4 TypedMethod
       │
       ▼
value-to-storage mapping: where is it kept?
       │
       ▼
Phi incoming to transfer requirements: what is transferred along the selected path?
       │
       ├─ normal edge → copy-dependency scheduling → required temporaries break cycles
       │
       └─ exceptional edge → save before the throw → receive at the handler entry
       │
       ▼
deliver the storage and transfer plan
       │
       ▼
StorageMethod
```

The three jobs together turn a type-explicit SSA value flow into storage accesses and a
transfer plan. P5 produces a plan, does not execute the method and does not emit Java
text directly. P6/P7 consume the plan and cannot re-solve copy dependencies, Phi origins
or type adaptations.

#### 5.5.2 Storage mapping and materialization

StorageBinding uses mutually exclusive states to express parameters, ordinary local
storage, pure literal views and initialization-related storage: an explicitly
uninitialized reference, or an evidenced natural merge deferred to materialization.
ValueId and LocalId are related by an explicit mapping, and their numbers need not be
equal.

Parameters bind their original parameters directly; ordinary definitions and Phi results
that need materialization first obtain independent storage and then choose a common
carrier under the reuse admission conditions. Pure literals are expressed through the use
views already determined by P4, and no variable is forced into existence for every
ValueId.
A producing operation with execution effects, such as a call, a field/array read or an
allocation, cannot be deleted because its result is unused, nor converted into an
expression executed again at several places.

An uninitialized object consumes P4's position-related states and evidence, and retains
internally the reference carrier produced by the allocation position, original alias
copies and commit relations. The internal uninitialized state does not mean the final
body must print uninit or allocate-instance; ordinary object creation whose execution and
storage correspond supports natural merge materialization and is expressed by default as
`new T(...)` in P7. When an independent position or state genuinely must be kept, an
explicit carrier is supported: it is neither treated as an already constructed ordinary
Java object variable nor a reason to reject the whole method.

P5 MUST retain the allocation result's commit on normal allocation completion, pre-
initialization aliases and the original state observable on failure paths. P5 MUST NOT
assign a default null to an uninitialized reference, materialize a normal initialization
result on an exceptional path, or merge allocation failure and constructor failure into
one storage state.
Ordinary reference uses and snapshots must not assume an uninitialized carrier has
completed initialization; the corresponding states and transfers follow P4's use facts,
and P5 does not re-derive initialization relations.

P5 does not fix final variable names or source lexical scope. Reuse candidates come from
original transfer relations, not from variable or debug names; equal types and connected
Phis do not mean equal values. Normal commits and exceptional branches on original state
transitions participate separately in liveness and interference analysis. Reuse moves no
instruction, definition, snapshot or edge copy; copy scheduling omits only Identity
self-copies that still satisfy their original conditions after reuse.
Storage not proven reusable stays independent, and local gaps are still retained. This
choice does not guarantee the fewest variables, the fewest copies or optimal performance,
and it adds no cross-stage feedback or independent runtime verification.

#### 5.5.3 Realizing Phis per original edge

P5 builds the destination, source and already determined adaptation of each incoming from
P4's PhiViews:

```text
P4: x3 = Phi(edge A:x1, edge B:x2), common representation int
                  │
                  ▼
P5: x1 → local1, x2 → local2, x3 → localJoin
    edge A: localJoin ← local1
    edge B: localJoin ← local2
```

A normal copy executes only after the corresponding edge is selected. Two edges with the
same destination are not merged by destination; the plan retains original transfer
identities and roles and does not place work at a uniform end of the source block or an
unconditional entry of the target block. P5 does not modify the original CFG; concrete
structural expression is completed by P6/P7 consuming the plan.

A cross-block use without a Phi accesses its existing storage directly and adds no
pseudo-entry Phi or copy.
P5 MUST accept pruned SSA's cross-block dominating definitions and cannot require the
source to be defined in the current block. Source availability comes from P3/P4; P5 is
responsible for establishing the materialization and use guarantees of the corresponding
storage.

If a Phi entry is associated with a P3 private StatePoint, P4 must retain its original
transfer and execution-obligation mapping along the product. P5 schedules the initial
entry, normal commit successor or handler receive from that mapping, rather than
re-finding a block head from the Phi's numeric ID or PC. A missing original mapping
cannot be treated as an empty copy.
A private state position appears in the storage product as an explicit transfer
obligation and adds no original CFG or effect.

Phi common representations, literal views and incoming adaptations are all produced by
P4. P5 does not add a cast, boxing or helper because a type is temporarily inconvenient.
Insufficient adaptation keeps a concrete result, and P5 cannot re-select a type itself.

#### 5.5.4 Copy dependency scheduling and cycles

The copy set on one edge has simultaneous-assignment semantics: all sources are read from
the old storage state and then destinations are written. "Parallel copy" refers to that
semantics, not to multithreaded execution.

P5 builds a source-read dependency index internally, recording which destination old
values are still needed by unfinished copies.
A ready queue selects destinations that can be overwritten safely; after a copy completes,
its source dependency counts and the queue are updated, and each selection does not rescan
all pending copies.

```text
simultaneous: a ← b, b ← c
sequential steps: a = b; b = c

simultaneous: a ← b, b ← a
sequential steps: temp = a; a = b; b = temp
```

When no copy is ready but dependencies remain, one needed old storage is saved into a
temporary variable of the same type, every pending read that needs it is redirected to the
saved value, and scheduling continues after dependency updates.
P5 MUST retain the old values of multi-use sources and copy cycles and must not replace
only one of the reads. The temporary's type comes from existing type facts and introduces
no new type solving.

A source may be a pure literal view already determined by P4 or an adaptation with no
observable effect. The dependency index MUST cover every storage the adaptation actually
reads; a literal source has no storage-read dependency, but its destination old value may
still be read by other copies. Redirection changes the carrier storage without erasing
original ValueId/UseView associations and without moving or duplicating an original
effect-producing operation.

Only an Identity self-copy over the same storage with exactly the same semantics may be
omitted. Equal LocalIds are not grounds for deleting different views or adaptations. Copy
step counts, dependency updates, temporaries and independent checks all consume request
resources, so the scheduling kernel's local cost must not be claimed as linear or optimal
cost for all of P5.

#### 5.5.5 Exception snapshots and handler entries

An exceptional transfer has no ordinary edge execution position at which an assignment
can simply be inserted. P5 schedules snapshots only for the exception joins and storage
transfers actually needed, and does not copy the whole register environment.

```text
before the throw position: shadow(handler, phi) ← old-value storage of this incoming
                      │
                      ▼
            execute the original operation
                   /       \
        normal completion   throws
                 │            │
        use the normal result ▼
                    handler entry binds the exception object per original facts
                              │
                    handler Phi storage ← shadow
```

The snapshot sits immediately before the corresponding throw position, and its sources and
interpretation come from TypedMethod. It may read only storage that is already available
and materialized by plan, or a pure literal view, and it must not add throws, calls, locks
or heap effects.
P5 does not make an uninitialized or unusable source satisfy snapshot requirements through
an arbitrary conversion.

Every handler/Phi that needs to receive keeps an independent saved storage. Each throw
position saves its corresponding incoming; the handler reads the saved value that actually
caused that exceptional transfer, not one global "latest register table". Multiple
handlers, nested handlers and throw terminals follow the same rule.
When the exception object is bound by the original handler entry, that definition order is
retained and the object is not defined twice.
An exception snapshot does not mean heap effects already performed are rolled back.

Entry copies, normal-edge copies and before-throw snapshots have different execution
positions and cannot be merged into "uniform end-of-block assignments". Default
initialization cannot repair a missing definition; synthetic storage initialization may be
retained only when existing storage-transfer facts establish that no actual use can observe
the initial value.
It is neither an original DEX definition nor added program behaviour for Java
compilability.

#### 5.5.6 Output and the stage's own checks

StorageMethod retains TypedMethod and adds:

- the ValueId-to-StorageBinding mapping, locals and temporaries;
- storage accesses at each use position and their P4 view associations;
- copy plans of the initial method entry and of every normal edge;
- before-throw snapshots and handler-entry receive plans;
- explicit uninitialized reference carriers, alias copies and evidenced natural-merge materialization plans;
- position-related states of each initialization use and concrete unavailability reasons.

P5 tests cover every required storage and use, checking that sequential steps are
equivalent to the specified simultaneous assignment, that temporaries are written before
they are read, and that each required exceptional transfer's snapshot matches its handler
entry.
The checks also establish the guarantee that storage materialization, copy interpretation
and original execution effects were neither moved nor duplicated.
They do not re-prove SSA construction or type solving and do not depend on P6 structure
recovery to complete the plan.

When P5 cannot establish some storage, copy or snapshot, it MUST retain that item's
original position, cause and affected transfers/uses and keep delivering the independent
usable plan; it must not disguise the item as successful with a default value or a skipped
copy. StorageMethod's coverage includes these local states, and one plan gap must not
cancel all storage.
All indexes, copies, allocations and test checks keep range, arithmetic and fallible
allocation guarantees.

### 5.6 P6: natural regions first, a Java-like control plan with explicit transfers preserved

Chosen route: exit-directed region construction driven by dominance/post-dominance and
loop analysis. Normal structured regions are formed first; parts that cannot be expressed
naturally keep a GraphRegion with labels and explicit transfers.
They belong to the same PlannedMethod, which neither re-executes method analysis nor
generates a dispatcher.

Java-like pseudo code is an accepted result notation. goto is not legal Java syntax; P6/P7/P9
MUST state its notation explicitly and must not make method delivery depend on Java legality
or how completely natural structure was recovered.
Insufficient structure recovery records concrete recovery diagnostics and does not
automatically become a whole-method Debug or Malformed result.
Existing input damage, internal invariant failures and resource termination keep their own
classifications.

Input: the execution graph, operations/terminals, typed uses, storage, edge copies,
exception snapshots, handler entries and initialization facts of StorageMethod. P6 trusts
these product guarantees and does not recompute Phis, use conversions or copy dependencies,
nor re-read original instructions to interpret control semantics.

#### 5.6.1 Region construction on the same execution model

```text
P5 StorageMethod
        │
        ▼
build graph indexes, normal structural relations, protection and lock constraints
        │
        ▼
original blocks/transfers as elementary regions; all known execution content is kept
        │
        ▼
compose local regions in exit-directed order
 ├─ Sequence / If / Loop / original Switch / LabelledBlock
 ├─ Try / Synchronized that can keep their scope
 └─ parts that cannot be composed safely keep GraphRegion + explicit labels/transfers
        │
        ▼
deliver position mappings of regions, transfers, copies, exceptions and locks
        │
        ▼
PlannedMethod → P7 method expressions → P9 text/JSON
```

P6 computes dominance/post-dominance relations of the normal graph, SCCs and natural loops
with their nesting internally. The dominator tree uses the Lengauer–Tarjan route and SCC
analysis uses an explicit stack; normal structure and exception/lock state analysis stay
separate and do not reuse dominance conclusions from P3's private exceptional commit graph.
Original blocks retain entry and order coverage through a unique BlockPlan; monitors,
protection boundaries and terminals may use intra-block positions. A WithinBlock position
between segments expresses only existing order and adds no DEX edge, operation, effect or
SSA value; the original EdgeId still reaches the original target block entry.

Post-dominance supplies later-position candidates and does not on its own prove that a
region can be composed. P6 checks
that region external entries, exits, loop relations and protection constraints hold, and
does not treat an SCC directly as a while.
Infinite paths that cannot reach the selected exit keep their original semantics, and no
runtime break or return is added just to compute post-dominance.

RegionContext records the current entry, later positions, nesting loops, continue/break
targets and exception/lock constraints. Child regions are built after exit ownership is
clear, and outer regions are composed afterwards. An explicit stack and a bounded work
queue are used, without host recursion or unbounded candidate search.

Branches, loops and switches may nest, and the whole method is not required to match one
fixed template. Common continuation content has a unique structural owner and is not
repeated inside every branch. A failed ordinary region construction stops only that
candidate composition and keeps the known blocks and transfers; content is not skipped
through a visited set.

#### 5.6.2 Correspondence of operations, edges and structural transfers

RegionPlan contains Block, Sequence, If, Loop, original Switch, LabelledBlock, Try,
Synchronized and GraphRegion. Every original reachable block and operation keeps a unique
execution owner; edges associate their original EdgeId, P5 CopyPlan, role and target
through TransferPlan.

A normal transfer can be expressed as sequential continuation, a branch arm, case/default,
break, continue, return or throw. A known transfer that none of these structures can
express uses an explicit goto, not a placeholder with an undetermined target. Conditions
and selectors come from existing use products and are not re-evaluated.

A normal edge MUST first select the original branch, then execute that edge's copies, and
finally complete the target transfer.
A label/goto likewise executes the copies already determined by P5; a bare jump must not
discard a join assignment. Different edges that reach the same target still keep their
original identities and copy relations.

```text
original edge E: B1 → B3, copy C
    ↓
B1: execute original operations → execute C → goto B3
```

A GraphRegion retains the complete block contents, ordered conditional transfers and
explicit exits. P6 retains explicit graph representation within the required common
scope and does not force a jump into already structured branch interiors.
Region composition must establish entry and exit correspondence; when composition cannot
hold, the plan keeps or expands to an existing graph region instead of duplicating original
effect-producing operations to fake natural structure.

P6 adds no state variables or synthesised while/switch dispatch and hides no jump behind a
helper. A switch from the original DEX, including a flattened switch produced by an
obfuscator, is still recovered normally; "no generated dispatcher" must not be read as
deleting original dispatch behaviour.

#### 5.6.3 Loop expression: while, do-while and for forms

A natural loop may choose while(condition), do-while, while(true), traditional for or
enhanced-for/for-each with exits judged at their original positions. A for-family header
may own an original initializer, update or iterator operation only when the delivered
operations, effect order and edge-copy positions are preserved.

A head condition suits while and a tail condition suits do-while; the form may be used only
when the evaluation count, original operations and edge-copy positions can all be
preserved. An established loop whose condition cannot be hoisted safely uses while(true)
and judges at the original position instead of moving the tail condition to the first
entry.

For-family presentation changes only expression form and deletes no original
initialization, update, iterator call or array access. If a header cannot preserve those
operations, retain them as ordinary body statements. continue and the back edge still
execute the corresponding update, condition evaluation and edge copies, and a do-while
continue must reach the correct tail test. A condition exit's normal continuation binds
that original edge's actual target and must not be replaced by a common post-dominating
position past the exit-side operations. A required labelled break/continue keeps its exact
structural target; an irreducible loop can still be retained completely as a GraphRegion.

#### 5.6.4 Notation for exceptions, locks and explicit jumps

Natural try/catch and synchronized are used only when every original throw position, handler
order, lock acquire/release and exception cleanup can be preserved. A catch-all does not
automatically become finally, and duplicated cleanup is not merged because the source looks
similar.

If a protection or lock scope cannot be composed naturally, P6 retains the explicit graph
of the affected scope, each throw position's ordered HandlerChain, exception commit rules,
P5 snapshots/entries and the original monitor operations. P7 consumes these facts to
generate explicit Java-like execution notation and does not guess Java scopes.

An explicit goto means jumping to the label after completing the corresponding edge's
transfer; on its own it executes no finally, releases no lock, rolls back no heap and
creates no initialized object. If natural structure would trigger those behaviours
implicitly, a cross-scope transfer must have been proven equivalent; otherwise the affected
common scope is retained as explicit graph representation.

A protection relation that cannot be expressed naturally MUST be shown explicitly as part
of the method expression, not only mentioned in a diagnostic that may be hidden. Pseudo
code describes catch relations with concrete operation positions, original priority and
handler labels, states the exception object's entry binding explicitly, and does not
disguise it as an ordinary goto.
Explicit monitor-enter/exit is notation for the original synchronization operation, not an
added runtime helper.

When P6 cannot prove a lock structure, it adds no synchronized and deletes no original
enter/exit.
Exception value sources and snapshots come from P5, and P6 does not reschedule save
semantics; each snapshot stays before its corresponding operation. The associated
uncertainty stays concrete and is not converted into a false natural protection scope.

#### 5.6.5 Complete delivery, diagnostics and implementation scope

PlannedMethod retains StorageMethod and delivers regions, normal transfers, protection
relations of throw positions, lock expression plans, and their local states and concrete
diagnostics. P6 MUST retain independently plannable parts and limit the impact of missing
storage, transfer or protection facts to the actual dependents; unknown control flow must
not be presented as an ordinary GraphRegion that pretends to be a recovered plan. P7
realises the plan directly, does not re-judge branch joins or identify loops, and does not
supply goto targets or handler relations in the printer.

P6 tests MUST cover original operations and transfers, ensuring that execution counts,
order, conditions/selectors, edge copies, exception snapshots, handler priority and lock
behaviour did not change because of structural composition.
Ordered handler chains and explicit jumps preserve control expression, but they are not by
themselves a whole-method correctness proof; guarantees for types, storage and semantic
conversion are still established by their own producers.

"Incompletely natural structuring" and "missing execution facts" are different states. The
former delivers a complete Java-like plan containing explicit transfers and does not trigger
whole-method Debug; the latter retains the missing ranges and concrete causes, and must not
build a complete plan from fake targets, default values or omitted blocks. Damage to the
input, resources and InternalInvariant follow §9 and are not concealed by the licence to
print pseudo code.

This plan learns from layered region construction, nested contexts and exit ownership in
JADX's normal mode, without copying its rewriting of the original CFG, pattern retries or
exception stubs. Structure recovery does not require the same appearance as the original
source and does not make Java compilability a completion condition.

The following defines the delivery surface of region plans and the target consumption
notation for P7/P9.

#### 5.6.6 Labels, region entries and transfer targets

P6 creates a method-unique LabelId for every entry that needs an explicit transfer. A
LabelId binds the original BlockId, the intra-block entry kind and the region owner, not a
source line number, text position or block traversal order.
An original jump still binds the original block entry; an intra-segment continuation label
carries only existing order within the same block.
P7 maps a LabelId once to an unambiguous method-local display name, and P9 prints only that
existing mapping. Original member names stay unchanged; a synthetic label promises no
stability across requests.

A label sits at the corresponding block entry, where the target block's original operations
and required entry receives have not yet executed.
The source edge's CopyPlan executes before the jump, and source-specific copies cannot
unconditionally move to the target label.
A handler label keeps the exceptional entry kind; an exceptional edge must not be replaced
by an ordinary goto, and no ordinary edge to the handler may be added to simulate an
exception.

| ControlTarget | Target meaning | Obligations before the transfer |
|---|---|---|
| Next | confirmed sequential successor | execute the selected edge's copies |
| Break(RegionId) | confirmed region continuation | execute exit-edge copies; original cleanup must not be skipped |
| Continue(LoopId) | the loop's exact continuation position | preserve update, condition evaluation and back-edge copies |
| Goto(LabelId) | explicit original block entry | execute that edge's copies, add no implicit cleanup |
| Return / Throw | original method exit or throw behaviour | preserve existing use interpretations, original prior operations and exception relations |

These are conceptual states, not a Rust enum that callers may construct freely. P6's producer
establishes the correspondence between targets and original transfers, and P7 does not guess
targets from text adjacency or the nearest loop.
Return/Throw need no invented normal EdgeId; associating with the original terminal identity
suffices.

P6's mixed representation is conditional on region-entry closure: an external transfer may
enter only an already exported region entry, and cannot bypass an If condition, enter a
natural branch interior or enter a body that already holds a lock implicitly. When an
internal transfer leaves a natural region, its target and implicit protection/release
behaviour must agree with the original edge.

If an unbridgeable cross jump exists between two candidate structures, P6 adds no new state
and duplicates no block. It stops composing those structures within the smallest confirmable
common owning scope and keeps a GraphRegion; when no smaller closed scope can be determined,
the whole method may be a readable label graph. Expanding an existing candidate tree changes
plan nodes only and does not copy, delete or re-analyse original operations.

```text
allowed: Sequence [natural region A, GraphRegion G, natural region C]
condition: every entry and exit of G corresponds to actual structural transfers of A/C.

not allowed: goto directly into the interior of a natural If's then, skipping the condition.
handling: retain the common scope containing the cross jump as an explicit graph; the
          condition and both blocks continue to be shown.
```

A GraphRegion retains explicit entry transfers, ordered blocks and all terminals. Its entry
is not inferred from "the first printed block", and layout does not decide jump meaning.
Several known edges may be shown, but branch roles or P5 copies must not be dropped because
source/target coincide. Only a Next established by P6 can omit the printed goto.

The elementary explicit graph supplies all available operations and transfers, and region
composition replaces plan nodes only after the corresponding conditions are established. One
non-matching candidate does not revoke other usable expression and does not restart method
analysis. Candidate replacement reduces not-yet-composed regions or pending transfers; all
traversals, allocations and correspondence checks keep range, arithmetic and fallible
allocation guarantees, and no unbounded search is performed to improve structural
completeness.

The following illustrates the target expression of known multi-entry control flow. It is not
compilable Java and not an actual output snapshot:

```text
void difficult(boolean enterAtB) {
    if (enterAtB) { goto B1; } else { goto B0; }
B0:
    workA();
    goto B1;
B1:
    workB();
    boolean again = repeat();
    if (again) { goto B0; } else { goto B2; }
B2:
    return;
}
```

The example omits edge copies that were not required; when the actual product has a
CopyPlan, P7 realises the copies inside each branch before jumping. The repeat call and the
condition that uses its result are different operations, and the call is not executed twice.
The method is not an empty stub or a raw instruction dump and still expresses existing
computation, calls and value flow.

#### 5.6.7 Notation for exception relations that cannot be wrapped naturally

Explicit protection notation uses method-local ThrowSiteId, HandlerChainId and handler
labels. P6 binds these identities to P2's original exception positions, selection positions
and ordered chains; P7 retains the association.
A chain is an ordered sequence of original descriptors and targets; it cannot be sorted by
type, no catch may be deleted because of a missing definition, and a catch-all cannot be
converted into finally.

The target display declares a site's chain with exception_map and shows the site anchor at
the original throw operation. It is execution-relation notation, not an ordinary Java
annotation, function call or runtime exception dispatcher.
A site anchor covers only the one original operation it is associated with and does not
extend the same protection to later goto or other operations.
Normal completion enters an explicit normal transfer; on a throw the chain is matched in
original order and the operation's normal register result is not committed. An unmatched item
propagates outward and adds no default handler.

```text
exception_map {
    S0 -> [Lexample/Failure; => H0, any => H1];
}

B0:
    shadowH0 = oldValue;          // before-throw save already delivered by P5
site S0:
    result = target.compute(arg);
    goto B1;

handler H0(exception e0):
    recovered = shadowH0;         // handler-entry receive already delivered by P5
    recover(recovered);
    return;

handler H1(exception e1):
    throw e1;
```

The Failure and compute references in this illustration stand for original facts only and
require no external class definition.
A handler's exception parameter expresses the original exception entry binding and does not
construct a new exception object.
If the original handler does not read the exception object, the product retains the
corresponding entry fact and adds no fake use.
When recover or rethrow has its own exception chain, that chain must be shown at its own
site; it must not automatically inherit the sibling catches of the same try, and the chain in
the table must not serve as default protection for the whole text range.

A site anchor belongs to an operation execution position, the snapshot precedes it, and the
normal result assignment completes only on success.
When one expression contains several real throw positions with different protection chains,
P7 MUST keep separate execution expressions instead of merging them into one large expression
whose protection semantics cannot be distinguished. A throw without an exception chain
propagates outward; special exception-selection positions such as monitor-exit are still
retained by the product and are not recomputed from printed positions.

A natural Try and an exception_map can each own only one kind of catch ownership at the same
site; both must not execute catches. If P6 cannot prove that the actual handler selection
agrees across a mixed scope, it retains that common scope as an explicit protection graph, so
that a Java try in the text does not catch extra handlers or gotos.
Explicit protection relations are retained in the method expression of the default text and
JSON, and do not depend on verbose diagnostics.

#### 5.6.8 Two expressions of lock scopes

A Synchronized region denotes original lock acquisition, release and cleanup that have been
proven to correspond. P7 consumes its mapping and does not search for a canonical template
again. Although the original lock operation is carried by natural syntax, its identity and
execution correspondence are still retained in the product. The acquisition link and every
SyncExit retain separately the original entry, release, subsequent target, and the P5 copy
references before and after release; a lock scope does not require one shared release point
for all paths. P7 MUST accept these obligations at the selected exits, and must not rebuild
links from syntax positions, move path-specific post-release copies or calls into the locked
body, or add a dispatcher.
Lock reference stability consumes existing P4 identities and P5 write positions; a static Root
or an allocation-source set alone does not prove the same dynamic acquisition.

Captures, handler receives, operations, branches and secondary exceptions inside real cleanup
still stay in their original protection region; natural synchronization carries only monitor
events that already correspond. When exits and copy positions cannot be preserved in existing
regions, P6 retains the complete explicit plan and the concrete reasons.

When a natural lock scope cannot be established, use the following explicit pseudo-code
operations:

```text
monitor-enter lockValue;
// original body operations and transfers
monitor-exit lockValue;
```

monitor-enter/exit is display notation for original operations, not an added helper. Required
throw site anchors and exception_map still bind these operations. Only a successful enter
acquires the lock; an enter that throws acquires nothing. Release and its exception rules
consume existing execution products and do not infer from textual nesting.

An explicit goto transfers control only and does not release a lock automatically. An original
release on the target path must still be reachable and execute; one on the source path must
execute before the jump. One lock operation cannot be carried by Synchronized syntax and also
execute as an explicit operation. When alias or scope evidence cannot be established, retain
the concrete uncertainty and the original lockValue instead of inventing a "lock is balanced"
conclusion.

#### 5.6.9 Minimum delivery surface from P6 to P7

PlannedMethod's read-only interface provides:

| Product fact | How P7 consumes it |
|---|---|
| RegionArena / root | build the same natural/explicit control tree without re-partitioning regions |
| Block contents and operation identities | use the unified operation-lowering logic to generate statements at existing order |
| typed uses and StorageBinding | realise value expressions and materialization without re-solving origins or types |
| TransferPlan | execute P5 copies and then use the already decided Next/Break/Continue/Goto |
| LabelPlan | convert an already bound target into a displayable name; no target is supplied from source layout |
| ProtectionPlan | generate natural Try or site/exception_map notation and retain ordered chains |
| MonitorPlan | generate natural Synchronized or explicit monitor operations; no release is added |
| original origins and recovery diagnostics | associate method, operation, site, edge and related region and retain concrete problems |

Construction-only RegionContext, dominator-tree work tables, candidate search and mutable node
ownership are not handed to P7. P6 may release that scratch only after delivering a closed
plan and must not hand P7 a half tree to complete.

P6's output tests verify execution ownership of every reachable original block, the actual
reached target of every normal edge, the protection chain of every site and the expression
owner of every lock operation. A natural region may execute many times dynamically; unique
ownership means only that no two duplicate static expression positions exist and must not be
read as the loop executing once.
The checks must include entry transfers, edge copies, early exits and non-terminating paths,
not merely node counts.

#### 5.6.10 P9 presentation and source positions

P7 expression nodes retain the original MethodKey, operation/terminal origins, EdgeIds, sites
and handler associations. P9 computes final UTF-8 byte spans while printing these existing
nodes, does not use original DEX PCs as source positions, and does not recover associations by
re-parsing pseudo code.

A goto's display target comes from LabelPlan; its origin associates the original transfer and
terminal, and the preceding copies associate the P5 CopyPlan and the corresponding incoming. A
label declaration associates the corresponding block entry. When loop, if or synchronized
syntax carries several original positions, P7 retains multiple origin mappings and invents no
single PC.

Both exception_map entries and site anchors associate original throw positions, handler labels
associate original entries, and monitor statements associate original lock operations.
Synthetic labels, saved storage and copies are not claimed as original DEX instructions. Origin
mapping is not a substitute proof of execution coverage or semantic correctness.

Natural structure and explicit graphs render the same MethodResult, and text/JSON do not select
different capabilities. Default output states the pseudo-code notation and the required
execution relations explicitly, does not stuff the whole method into comments, generates no
"Method not decompiled" exception stub, and does not silently skip complex blocks or treat them
as a successful empty method.

### 5.7 P7: MethodResult

Input: PlannedMethod, whose mature operations, bindings, types, storage, regions and required
context are consumed through formal interfaces. A member with confirmed absence of code
consumes the P0 declaration and the method reader's no-code facts along the independent
declaration path, fabricates no DecodedMethod and enters no P2–P6 stage.

#### 5.7.1 Default per-operation expression

P7 lowers natural regions and GraphRegions with the same set of operations, use views and copy
logic. The only difference is the delivered control nodes; no independent graph-method
decompilation entry, register interpreter or fallback SSA is created. Each operation is lowered
once at its own static owner position and the region node references the corresponding
expression; the implementation does not generate one natural body first and rerun a graph body
after failure.

By default P7 generates statements along the existing execution plan and retains the required
temporaries already scheduled by P5. Results of calls, field/array reads, conversions and
computations are produced at their own positions and used by later statements; operations are
not folded into nested expressions merely to shorten the code.

```text
existing executed operations:
    x1 = callA()
    x2 = callB()
    x3 = Add(x1, x2)
    return x3

default expression sketch:
    int t1 = callA();
    int t2 = callB();
    int t3 = t1 + t2;
    return t3;
```

The example does not constrain actual variable names. Pure literal views already determined by
P4 may enter a use expression directly, and no variable is required for every constant; an
unused result also must not make its original effect disappear.
Per-operation does not mean one forced line per original instruction: a P2-bound producer/result
and initialization/operation expansion with explicit evidence are still expressed under their
own execution-correspondence rules. When the association and execution correspondence between
an ordinary allocation and a constructor call hold, `new T(...)` is synthesised by default
instead of printing low-level initialization notation to keep instruction-line counts. Original
evidence is retained internally and need not be reprinted in the body.

Expression composition is allowed only under concrete semantics-preserving rules; "used once,
so it may be inlined" cannot be the whole basis. Every composition MUST preserve input-read and
result-commit order, execution count, value interpretation, exception positions and handler
selection, lock and initialization order, and must retain P5 copies/snapshots and P6 transfer
handling. Real throw positions with different protection chains must not be merged into one
expression that cannot express their separate exception relations.

When a composition rule cannot establish these guarantees, step-by-step expression is retained
without re-analysing the method, changing types, reducing original effects or downgrading the
method to Debug. A rule may change only P7's expression product and must not modify existing
P2–P6 facts. P9 cannot inline, merge or reorder these expressions again.

```text
PlannedMethod
    ↓
fix label display names, retain type references and original member names
    ↓
lower operations/use views/P5 copies uniformly, retaining sites and original origins
    ↓
assemble If/Loop/Try/Synchronized/GraphRegion and the base emission layout per the P6 plan
    ↓
P7 compose consumes existing position, value and copy facts to form the single final emission plan
    ↓
encapsulate the owned MethodResult
```

#### 5.7.2 Object allocation, initialization and constructor expression

P7 consumes P4 initialization facts, the P5 storage/materialization plan and P6 execution
positions. When the allocation, constructor association and execution correspondence of
ordinary object creation are established, P7 MUST use `new T(...)` by default.
The fact that DEX splits allocation and initialization into two instructions is not by itself
grounds to claim that no natural Java form exists.

```text
original behaviour:
    uninit<T> obj = allocate-instance T;
    init-direct obj, Lexample/T;-><init>()V, [];
    use(obj);

default natural expression:
    T obj = new T();
    use(obj);
```

This ordinary example has no expression problem of its own. The original allocation and
initialization identities are still retained in the method product's
origins and correspondences, which does not mean the two must be printed separately. Explicit
initialization pseudo code is used only when an independent execution position, state or other
relation that natural new cannot carry accurately must be retained.
A natural merge condition that does not hold is not a failure gate for method analysis or the
readable result.

| Original behaviour | Expression obligation |
|---|---|
| new-instance | allocate the object at the original position, commit the destination reference only on normal completion, retain type/initialization-related effects and the exception site |
| copy of an uninitialized reference | copies the same object reference, allocates nothing again and assumes no completed initialization |
| `<init>` executed on that reference | calls the original constructor with the original receiver/argument order and obtains the initialized state along the corresponding path after normal completion |
| constructor throws | retains the exception relation and the not-yet-completed state without rolling back effects the constructor already performed |
| this/super initialization call | retains receiver origin, original target, arguments and call position without reordering prior operations for Java syntax |

When independent expression is genuinely required, initialization uses the following Java-like
notation, which is not legal Java, an added helper or another interpreter. Types, receiver and
target references all come from the analysis input:

```text
uninit<T> obj = allocate-instance T;
int arg = computeArg();
uninit<T> alias = obj;
init-direct obj, Lexample/T;-><init>(I)V, [arg];
use(alias);
```

`allocate-instance` expresses the original object allocation, runs no constructor and is not
malloc.
`init-direct` executes the specified constructor on the same object, and its V return produces
no replacement object.
`uninit<T>` marks the reference state between allocation and initialization; after the
initialization call completes normally, aliases of that object consume the initialized state at
their use positions, without implementing the state change through a new allocation or by
redefining every SSA value. Equal static identity does not mean every dynamic path is
initialized.

When allocation or initialization has exception protection, P7 retains the site and the
corresponding chain at their original positions.
When allocation succeeds but initialization fails, the already committed reference and
observable effects remain: they are not overwritten with null and the post-initialization state
is not delivered to the failure path. A use that genuinely cannot be proven keeps its problem;
printable explicit pseudo code must not be used to claim that it satisfies the use constraints
of an initialized reference.

Ordinary creation with arguments likewise uses natural expression by default once original
execution correspondence is established. For example, when the argument value is produced
before allocation and the alias and use between allocation and initialization can be carried by
one expression:

```text
original operations:
    arg = computeArg();
    obj = allocate-instance T;
    init-direct obj, Lexample/T;-><init>(I)V, [arg];
    use(obj);

evidenced natural expression:
    int arg = computeArg();
    T obj = new T(arg);
    use(obj);
```

The merge conditions include the same object, exact constructor reference and arguments, one
allocation, original input-read order, result commit, alias storage, initialization order and
exception positions. A normal `new` assigns outward only after the whole creation expression
succeeds; if a failure path must observe the original reference already committed after
allocation, P7 cannot hide it behind that assignment. When protection relations or intervening
content genuinely need independent positions and the existing natural merge rules cannot carry
them accurately, the explicit operations are retained without widening the search or re-running
upstream analysis. The judgement rests on actual product facts, and these problems cannot be
assumed out of thin air for ordinary creation.

The runtime order of Java ordinary object creation is: allocate the object, evaluate constructor
arguments from left to right, call the constructor; see
[JLS §15.9.4](https://docs.oracle.com/javase/specs/jls/se8/html/jls-15.html#jls-15.9.4).
Therefore "arguments are computed after allocation" does not by itself prove that
`new T(computeArg())` changes order. Whether merging is possible still requires establishing the
original site, effects, commit and alias correspondence, and cannot be admitted from this surface
order alone. This plan expresses operations individually by default and does not automatically
inline argument computation with intervening effects because of that Java evaluation order.

When this/super calls can be expressed naturally in their original order, `this(...)` or
`super(...)` is used. If independent operations precede initialization or the source syntax
cannot carry the original order, constructor pseudo code is retained:

```text
constructor T(int arg) {
    // pre-initialization operations, expressed from existing facts
    init-direct this, Lexample/Base;-><init>(I)V, [arg];
    // post-initialization operations
}
```

Here the target and receiver are determined by existing products and are not guessed from names
or an external hierarchy.
P7 MUST NOT move operations, hoist computations, add helpers, default-initialize references or
withdraw the method in order to put super first in a Java constructor body. Delegation targets
and related unknowns or contradictions keep concrete facts, and inability to express naturally
must not be treated as a missing constructor or an invalid original method.

Natural and explicit expression share the same initialization-operation product; P7 tests check
the correspondence of the chosen expression and do not run their own complete register/alias
tracker. P9 prints only the selected expression and original origins, and P8 does not recover
constructors again when assembling the class or add constructors or initialization writes to
make the class compile.

#### 5.7.3 Expression coverage of supported operations

The capability requirement of this plan is: every execution operation already supported by P2 has
a corresponding natural or explicit pseudo-code expression rule in P7. After P3–P6 deliver a
contract-complete PlannedMethod, P7 cannot withdraw a method result merely because no natural
Java form or no expression branch exists.

```text
execution operations already supported by P2
          │
          ▼
P3–P6 complete their own processing and deliver required facts and explicit states
          │
          ▼
P7 has a natural or explicit pseudo-code expression rule for every operation kind
          │
          ▼
a complete readable method result
```

"Complete expression coverage" means that recovered expression and explicit unrecovered fragments
together account for original coverage, not that every semantic item was recovered. When required
value sources, type interpretations, storage or control facts are missing, P7 MUST retain the
corresponding ranges, causes and independently completed content and must not disguise them with
generic pseudo-code nodes as if semantics were recovered.
Resource and invariant errors keep their classification and their scope follows §9; a state name
does not withdraw all expression, and a coverage requirement does not make a damaged construction
state usable as an AST.

When operation semantics are already complete but P7 does not yet implement their expression, that
is a general capability gap to close, not a property that makes the operation undecompilable.
Capability migrations MUST check coverage from operations through inputs/outputs, uses, storage,
transfers and final expression together; adding P2 recognition alone cannot claim end-to-end
support.

Ordinary calls use natural method expression by default; for example, a virtual call of the
original `Ljava/lang/String;->length()I` is expressed as:

```java
int t2 = t1.length();
```

The original owner, name, prototype, invoke kind, receiver, argument order and origins are still
retained in the method product, and the complete reference need not be reprinted in every body.
A missing target class definition does not prevent that expression; unknown source binding retains
concrete uncertainty without switching the target or adding a runtime conversion.
Ordinary field and array accesses likewise prefer natural notation. Explicit pseudo code is used
only where a natural form genuinely cannot carry an original operation relation, and ordinary
operations are not lowered for conservatism.

#### 5.7.3.1 Original-target signature qualification

P7's call expression MUST retain the original logical DEX reference identity, owner, name,
complete prototype and invoke kind, and must not re-select the target from displayed argument
types. Original dispatch kinds such as virtual/interface are retained; an original reference does
not equal an already solved concrete runtime implementation.

Target-signature qualification in a Java-like call is binding information, not generic type
arguments, argument casts or a runtime operation. The following two lines retain different
delivered original call references:

```text
Api.register<(Listener)>(h);
Api.register<(Handler)>(h);
```

The qualifier does not change h's value, type origin, evaluation count, initialization state or
call order, introduces no check-cast, helper call or new exception position, and does not turn an
otherwise unusable origin into a usable one.
When the target prototype cannot be read completely, P7 MUST retain the original reference and the
concrete unknown rather than guess a parameter signature.
The qualified form is part of the call expression's semantics, not a comment substituting for an
incorrect call target.

The internal complete reference includes the return type. P7 chooses parameter-list qualification
`method<(ParameterTypes)>` when a reference difference must be disclosed, and chooses
`method<(ParameterTypes) -> ReturnType>` when original references share a parameter list but differ
in return type. Other cases that must state the reference explicitly keep the complete signature.
Type names are planned by P8, and P9 does not print quoted DEX descriptors directly; a type that
cannot be named naturally keeps its existing explicit form.

For a qualified instance call whose receiver's existing nominal declared type cannot accurately
supply the original owner, P7 chooses `(receiver via Owner).method<...>(arguments)`. `via` denotes
the owner of the original method reference, not the class where the runtime value was declared, its
runtime object type or the selected concrete implementation.
It introduces no conversion, check or new evaluation; virtual/interface dispatch is unchanged.
A static call states its owner as `Owner.method`. A super call still expresses original dispatch as
`super`; when its owner must be disclosed it is written
`(super via Owner).method<...>(arguments)`. Constructor delegation continues to use
`this<...>(arguments)` or `super<...>(arguments)`, and explicit initialization of an allocated object
prints the selected signature after `<init>`.

Duplicates or ambiguities in the original namespace likewise cannot be hidden by shorthand. P7
determines the qualified form from delivered facts, P8 only realises the existing class-name/import
display plan, and P9 only prints the existing expression without solving overloads or re-reading
declarations. A default natural call prints only the determined receiver, member and arguments; the
original reference identity, invoke kind and call prototype are retained in the owned reference
records and origin spans and are not repeated per call. A displayed argument type that merely widens
towards the parameter type is not an ambiguity. polymorphic/custom calls retain their own original
references, independent call prototypes and call-site/handle identities, and qualifiers must not
rebind them as ordinary calls.

Natural calls remain the default display; signature qualification is used when the original target
must be stated, and neither generating compilable Java nor influencing Java overload selection
justifies inserting automatic casts or type aliases.
When the original DEX already has a check-cast, P7 MUST retain its check, resolution and exception
semantics and cannot replace or omit it with signature qualification.

P7's expression rules MUST preserve operation inputs, normal results, effects and exception
positions, and cover operations whose results are discarded and every terminal. An unused result is
no reason to delete a call, field/array read, cast or division. P2-bound producer/results and
ordinary object creation are expressed under their own complete correspondence, and original
instructions need not equal source statements one for one.

An implementation SHOULD constrain coverage through exhaustive branches on operation kinds and must
not use a wildcard that turns unhandled operations into empty statements, NOPs, default values or a
printable string. Display rules belong to the existing P7/P9 responsibilities and add no second
instruction-decoding, type-recovery or whole-method retry route.

If an implementation still has an expression gap, it retains that position's original identity, known
operations, result associations and concrete unfinished cause together with the other expressions
already formed; it must not claim complete semantics or blank the whole method.
Before this capability migration is complete, expression for already connected operations must be
filled in, and the gap must not be treated as a normal exit.

#### 5.7.4 Method product and result guarantees

P7 consumes the transfer targets and scopes already decided by P6 and only generates the
corresponding AST; it does not re-identify loops, compute join points, choose handlers or schedule
lock releases. Literals, conversions and temporaries are expressed from existing P4/P5
products and must not re-guess types or re-materialize initialization results because of goto
notation. When an original member name cannot use normal Java spelling, the original
identity is retained with explicit original-name notation instead of a global rename or
withdrawing the whole method.

P7 constructs the method expression; tests separately check the conversion of operations,
values, copies, control, exceptions, locks and initialization. Tests do not re-run P6's region
algorithm or another upstream construction, and do not trust AST counts alone.
P7 MUST NOT make up for missing decoding, SSA, type or region analysis. P7 MUST accept the
GraphRegion, labels/explicit jumps and protection notation already delivered by P6 and must
not withdraw an existing method expression because they are not legal Java. The method body
states its Java-like notation and generates no fake exception stub.

MethodResult owns the original MethodKey, declarations and expression nodes, type references,
label table, protection expressions, required origins and concrete uncertainties; this data
borrows neither Engine, PlannedMethod nor a DEX lease. Type uses retain original reference
identities, descriptors and use positions and are not frozen early into final type spellings
that could be replaced by string manipulation; P8 forms the class-scoped name plan in one
place.

P0 shares the owned contents, coverage and problems of original method/parameter annotations
through read-only MethodMetadata; the method reader/P1 delivers it along the single chain. The
dynamic reference reader/P1 retains call-site bootstrap argument events, original bytes and the
leaf declaration pool, and P7 only copies existing facts without re-reading the input.
ExpressedMethod privately retains this execution's P6 product for MethodInspection to observe
by borrow; into_result releases the retained analysis and returns an Arc<MethodResult> with no
input lifetime.
P8 and same-execution observation share the immutable owned result without copying the
expression arena; an unpublished expression prefix is still delivered as ExpressionPartial and
cannot be turned into a mature MethodResult.
The observation interface hands out no owned predecessor and offers no construction or solving
entry.

Established method-analysis scratch is released once it is no longer consumed. P9 renders with
existing expressions and the explicit name plan and does not switch representation mode between
text and JSON, generate new jumps or re-read reference declarations. package/import arrangement
does not re-run P7 and does not modify operations, types or call identities.

P7's EmissionPlan fixes final emission positions; the original control/outlet table continues to
retain origin facts and is not a second executable schedule. Both original edges of a loop
condition are selected inside the predicate and carry their copies, and an independent break
does not pass through the condition-exit edge's copies. A syntax-owned header/predicate does not
emit a separate continuation, avoiding skipping its own condition or monitor acquire. When
natural construction spans nodes, continuation resumes at existing initialization points rather
than falling in an assumed PC order.

A matched synchronized release, post-release copies and routing terminals emit at their original
nodes. The end of a with_explicit_exits container releases no extra lock; a release ends only
that lock scope and does not remove an enclosing try/catch. P9 prints these established positions
and explicit semantics directly and adds no control analysis.

The method as a whole uses Java-like notation and adds no "legal Java/verified" completeness
badge. When goto, exception_map or explicit monitor appears, the notation identifier and the
execution-relation body are visible by default; hidden recovery diagnostics cannot be the only
carrier of those relations. Labels/goto do not change the result's error classification, and
real damage and invariant errors stay with the result.

The body of `<clinit>` is still recovered by the method chain, and class assembly must not
re-analyse that method.

## 6. Stage trust and transformation obligations

Pn's analysis algorithm trusts the guarantees declared by the Pn−1 product, does not repeat
upstream admission and does not repair upstream results.
Ordinary decompilation MUST NOT run independent output verification, symbolic replay, duplicate
graph/dominance checks or formal proofs.
That code MUST compile only under `cfg(test)`; no runtime or feature switch may be added to
ordinary requests.
Existing independent checks stay in the test configuration; by operator decision no test case
exists now, and no tests may be written later. Independent checks are not a gate on product
publication or readable output.

Safe parsing of original input, range/arithmetic checks, fallible allocation and required
construction error handling remain. The solving, origin binding and region-match conditions an
algorithm needs to establish facts are not an extra acceptance process; this section must not be
used to delete required algorithms. Stage construction and its tests are owned by the same
module, per §4.3.

The transformations each stage actually performs must still satisfy the following semantic
obligations:

| Owner | Check obligation |
|---|---|
| Method reader | original method provenance, safe section and record reads; independent metadata records |
| P1 | disassembly coverage, operand interpretation and the original instruction associations consumers require |
| P2 | correspondence of original instructions to operations/effects/commit rules/edge roles, including exceptions and origin coverage |
| P3 | exception-aware state mapping, pruned SSA definitions/uses/Phis, wide values and edge transfers |
| P4 | type constraints, fact selection and use conversions |
| P5 | value-to-storage, per-edge out-of-SSA, copy ordering, exception snapshots and materialization |
| P6 | natural regions and explicit graph transfers, copy handling, complete correspondence of original protection/lock expression |
| P7 | execution correspondence from mature analysis products to method expression |
| P8 | correspondence of original members, class preparation and method results to class representation |
| P9 | correspondence of existing representation to text/JSON and spans |

Independent transformation verification cannot trust builder counts or boolean results alone.
Replay that needs original evidence keeps its independent obligation and reads through a
restricted interface; it does not authorise an analysis implementation to build another
recovery route.

Existing original replay and transform checks MUST have their obligations, evidence and owners
traced item by item during migration. They cannot be deleted under "trust upstream", nor moved
mechanically to P1; P1 is not responsible for proving SSA, type or source transformations that
have not been produced yet. Check migration must record the owner and evidence obligation and
implement it within the approved target under the root authority, without widening stage
permissions or changing output/scheduling agreements.

A bugfix locates the first violated point: a code-item section/record read error belongs to the
method reader, an opcode/width/operand interpretation error to P1, an operation/graph error to
P2, and a binding error to P3.
When an output contract lacks a fact that real consumers need, fix the producer and consumer
contract instead of re-reading original input downstream to hide the gap. When an upstream
product contradiction is found, stop the affected processing and report the concrete position;
neither continue with a fake success after repairing it nor attribute every internal violation
to damaged input.

## 7. Declaration context, lifetime and concurrency

A class request owns ClassFacts, and method entries and owner views borrow that product.
MethodSource borrows original DEX sections and required class facts, is handed to P1 inside the
method worker, and P1–P7 products retain the required evidence. Read-only context continues
through product interfaces without cloning all class facts.

The lifetime of original DEX bytes and the lifetime of P0 class facts have different sources and
are not forced to be equal.
Only types that genuinely borrow that data carry the relevant lifetime. Method results and the
final report own the data their output needs, do not borrow the whole analysis scratch backwards
and do not form self-referential reports.

The input lease lasts until original reads and required checks end; ClassFacts lasts at least
until method tasks and class assembly finish. Borrowing copies no original data, but retention
time affects peak memory.

Declaration queries locate and read in-input facts by original identity. The current class reuses
P0's produced result; other classes reuse the same declaration producer on demand, without
requiring every class to be collected first. A query must not interpret "not found" in a partial
directory as a non-existent member and must not offer method-analysis construction authority.

Engine uses scoped Rayon method-level tasks, keeping read-only sharing and independent method
state. The active concurrency configuration, pool-startup failure and result-joining contracts
are in [Engine method scheduling](phases.md#current-method-scheduling).
Completion order decides neither member identity nor original program order. Allocation failure
and backpressure keep their existing classification and behaviour; no numeric quota or request
cancellation mechanism is added.

### 7.1 Borrowing class facts and concrete lifetimes of method products

The current class request obtains immutable ClassFacts before method tasks start; all tasks use
the bound entries produced from that object. ClassFacts holds no reference to Engine mutable
state or to its own fields.
Method entries borrow P0 declarations and required owner facts, and the whole field/method
directory is not copied per method.

The method reader's MethodSource has two borrow sources: DEX sections with `'dex` and class facts
with `'class`. Preceding products store no thread worker or mutable declaration-query state;
the original declaration reader's restricted access capability is independent of analysis
products and provides no request-cancellation parameter.
The restricted reference-read capability P1 needs is delivered by the scheduler and is not
placed in an arbitrarily callable ProgramIndex for later stages.

Method products retain required borrows internally after being moved, so a task's lifetime
cannot exceed those two sources. Scoped Rayon execution ends the borrows before join returns, so
tasks need not be `'static`; neither Arc<Mutex<ClassFacts>> nor a copy of the whole DEX is added
to avoid a lifetime design.

P7 forms the owned MethodResult and copies declarations, descriptors, literals and original
facts as the actual output needs; the copy uses the same resource ledger. The result borrows no
preceding product, and method scratch is released once the last consumer finishes. Class
assembly moves these method results and keeps no reference to its own members inside
ClassResult. P8's name plan associates by original use identity and creates no self-reference.

The final report retains only facts and expressions that must be displayed and does not have to
keep every P1–P6 analysis graph permanently.
Output resource exhaustion is not resolved by implicit truncation; facts not yet delivered
cannot be claimed complete.

### 7.2 Restricted declaration queries and the single production of the current class

The declaration module exposes only original-fact reads and read-only directory queries with
coverage states; it offers no method body, CFG, SSA or type-solving entry. Every reference
retains its original logical DEX identity and original UTF-16 descriptor, and type projections
retain the root reference and rank without inventing new original IDs.

P4's class-level context divides into: the current class declaration delivered with the method
product, original signatures consumed directly, and known in-input definitions read when a
concrete behaviour-relevant dependency requires them. An ordinary reference read does not need
assignment compatibility proven first, and the call target is not re-selected by that query. The
returned state distinguishes present, missing definition, ambiguous and local directory
problems.
A missing target definition does not erase the original reference signature; no "default
public/nonvolatile" declaration is created.

A current-class query borrows this request's read-only ClassFacts declaration view directly and
does not materialise the same class again through Context. Context locates other in-input class
definitions and reuses the same P0 declaration producer on demand; the cache holds immutable
facts rather than a second hand-built directory. That production query is composed by the input
context layer and does not give P4's implementation authority to call P0 analysis construction.

The current superclass relation query uses only class-header facts already delivered for this
logical DEX. P0's private OwnerContext reuses the current OwnerFacts and the input layer's shared
declaration view and travels with the method product; P4 consumes only Same, Ancestor, Unrelated
or Unknown with a concrete gap and receives no directory, code or stage-construction authority.
Hitting the original superclass reference establishes the relation without a target definition;
ascending further requires a unique definition within this DEX. Ordinary current-class/direct-
superclass checks build no class position index, and only the slower ancestor query builds a
shared index on demand and reads the required class headers; no full ancestor closure is
pre-built and no other DEX is queried.

Definite malformed/unsupported read results may be cached under existing rules; resource and
invariant failures must not be cached as "class does not exist". Cached queries keep original
identity and range guarantees.
This plan adds no cross-request mutable method-analysis cache and does not merge different
original definitions with the same descriptor. A partial directory cannot establish absence or
hierarchy uniqueness.

## 8. P8 class preparation and assembly, P9 rendering

P8 owns class-header/field expression, static preparation, member presentation ownership,
class-scoped package/import name plans and class assembly. Its final input is ClassFacts, all
method results and required class-level evidence, and its product is the owned ClassResult.

Assembly is based on the original member directory, not on the list of successful methods.
Failures, declaration-only members and generated members retain original identities; when the
directory is incomplete, actual coverage is reported and all members are not claimed known.
Original encoded preparation is expressed before the original `<clinit>`, and no initialization
or constructor is added for javac.

Class-level preparation and final assembly happen at different times:

- Preparation that depends only on P0 may complete before method tasks.
- Evidence that affects method recovery but depends on code or declaration constraints must
  first be established by its stage producer.
- Presentation ownership and coverage that depend on final method results are completed during
  assembly.

A P8 implementation MUST NOT call P1/P4/P7 construction entries, nor parse method code itself.
Engine schedules those entries according to required products. Existing prerequisites of
generated members and captures must be stated item by item before migration; decisions cannot be
deferred to preserve a surface linearity, and no path that re-runs methods may be added.
Without established evidence, members must not be hidden from name or flags alone.

### 8.1 Class-scoped package arrangement and import plans

Package arrangement means unifying package, explicit imports and type display names within the
output scope of a single class; it does not merge analysis identities of different packages,
classes or DEX images, and it does not organise a multi-class output directory.

```text
P0 class header/field facts + structured type references of each P7 method
                          │
                          ▼
P8: collect type uses → resolve display-name conflicts → form TypeNamePlan
                          │
                          ▼
ClassResult: package + explicit imports + mapping from original references to display names
                          │
                          ▼
P9: print the existing AST and the name plan
```

The current class's package is determined only from the package path of the original descriptor and
appears once in the class output. Source file, selectors and references from other methods cannot
rewrite the current class's package name.

P8 collects type uses in class headers, fields, method declarations, method expressions and
annotation expressions. P7's AST keeps type references rather than finished rendered strings; P8
MUST NOT delete package prefixes by whole-text string replacement, which could modify literal
strings, same-named types, original descriptors or diagnostics.
Complete descriptors and method signatures inside pure original facts are kept as they are and are
not abbreviated by imports.

TypeNamePlan records type spellings by original type identity and concrete use position. Original
references still keep their logical DEX index space and are not merged by short name. Repeated use
of one display target produces one explicit import; different original references with the same
complete spelling may share that import text but must not merge original references, declarations
or analysis facts. A defined ambiguity keeps its concrete problem.

Name selection rules:

- Same-package and java.lang types use their short name when there is no known conflict and the
  corresponding use can be abbreviated unambiguously.
- Other types use an explicit single-type import and are shortened at uses that can be
  abbreviated unambiguously.
- No wildcard or static imports are generated, and package members outside the input are not
  enumerated.
- Different types with the same short name, the current class or known nested declarations, or
  shadowing use scopes keep the required fully qualified name; different uses of one type may be
  spelled differently and the short name is not forced globally.
- Missing facts cannot be treated as proof that no name conflict exists. When a short name cannot
  be established, the complete spelling is kept without loading external declarations. A complete
  spelling also does not by itself prove Java name resolution or access legality.
- Nested type spellings consume only existing ownership facts and do not split the owner at `$`;
  a top-level class whose original name contains `$` keeps it. Original members are not renamed for
  import convenience.

For example:

```java
package example;

import java.util.List;
import java.util.Map;

class Case {
    List items;
    Map index;
    String name;
}
```

When both java.util.Date and java.sql.Date are referenced, each keeps its fully qualified name; no
conflicting Date import is generated, and the conflict is not resolved by renaming a type or adding
a cast.
Actual uses include static call qualifiers, not just field types; P8 handles known shadowing in the
concrete use scope and must not replace all expressions globally after counting only type
declarations.

P8's name tests confirm that each spelling still associates the same original reference, that
imports have no duplicates or conflicts, and that literals, original descriptors and member
identities were not rewritten. They solve no new type, overload or dispatch semantics; when an
ordinary Java spelling cannot be established, the complete original-name notation and the concrete
problem are retained, and name arrangement failure does not become an output gate for the whole
class.

Import output order promises no stability; the deduplication facility serves only the current
class's name plan and adds no global sorting or waiting for method completion order. Reference
collection, name-plan allocation and output keep range, arithmetic, fallible allocation and
backpressure guarantees and introduce no numeric quota.

### 8.2 Display product delivery for P7/P8/P9

P7 delivers the owned MethodResult with type references separate from operation semantics. P8 forms
the owned TypeNamePlan and keeps it in ClassResult together with the class header, fields, static
preparation and each method result.
The display mapping associates through original references and use identities and does not modify
the execution meaning of the P7 AST.
P8 MUST retain recovered expression and unrecovered fragments inside a method and cannot fold a
mixed method result back into whole-method unavailability. Name or ownership problems are recorded
per original reference/member and independent code continues to assemble; when a planned name
cannot be adopted, P8 retains the existing original display facts and local states for rendering.

A single-method request still does not require assembling a complete class. Engine may call the same
P8 name-planning capability to form a standalone display plan for the selected MethodResult; without
class-level package/import output, fully qualified names are kept, the whole class's short-name
environment is not guessed, and no other method is analysed again.

P9 consumes existing MethodResult/ClassResult and the name plan, outputs text/JSON and generates
final UTF-8 spans. P9 does not parse input, select types, supply semantics, decide imports or shorten
names; text and JSON use the same product and display plan. Source positions are computed from the
actual output, and byte-length changes caused by short type names cannot reuse previously generated
spans.
P9 MUST print recovered expression, unrecovered ranges and concrete causes according to node state,
and must not reject the whole body because one fragment is unavailable, nor re-parse types or run
analysis to restore the display.
When basic output resources or the target fail, return a concrete delivery error; mid-way truncation
is not a successful partial recovery.

Engine calls P9 with a RenderTask. Rendered owns UTF-8 text, half-open byte spans, concrete rendering
problems and the same immutable source product; write_text/write_json use that one text.
A method-private traversal/display-index scratch failure is limited to that member; when shared
output or class-level display context cannot be established, a RenderStop is delivered that retains
the source product and the completed prefix and is not relabelled a success.

### 8.3 Member contract of class assembly

P8's input member set is associated with P0 original FieldKeys/MethodKeys, not matched by name,
signature display text or worker completion order. Every located member in the field and method
directories has an independent declaration, expression or concrete failure result; pseudo code
appearing in an expression does not affect assembly of the whole class.

ClassResult contains:

- the original class identity, the available class declaration and related local problems;
- field identities, declaration expressions and original static ordinals;
- original static preparation distinguished from implicit defaults and explicit encodings;
- the results of all known method members, including constructors, initializers and no-code declarations;
- existing class-level evidence, member presentation mapping, TypeNamePlan and concrete diagnostics.

Assembly may build original-key indexes internally but creates no sorting wait queue. Duplicate
member results, mismatched owning classes and a known directory entry with no result are concrete
association invariant errors and are not silently skipped.
When the P0 directory has only a prefix, P8 retains the insufficient original coverage and does not
claim class completeness from output counts.

Encoded static preparation is expressed by original static ordinal and placed before the original
`<clinit>` executes. P8 does not move writes out of constructors or `<clinit>`, does not delete later
overwrites of prepared values, and does not add initialization to satisfy Java final/constructor
legality. P0 scalar/encoded facts are the only basis; choosing a readable expression belongs to P8,
decoding original bytes does not.

### 8.4 Migration constraints for existing class-level evidence and prior scheduling

The old `engine/member_emission.rs` and `engine/constructor_chain.rs` are archived migration-review
material, not current scheduling entries; they cannot be moved wholesale into P8 or turned into a
new method-analysis entry.
When the current chain has not produced the corresponding ownership evidence, P8 retains original
members and does not restore the old analysis route.

| Item | Original/analysis fact producer | Class-level or expression consumer | Migration obligation |
|---|---|---|---|
| Class header, fields, member directory | P0 | P8 | consume directly, do not re-visit class-data |
| Signature/Throws/nested annotations | original metadata reader/P0/P1 | P4 declaration interpretation, P8 class expression | retain original text and local states; do not treat missing metadata as absent members |
| Current method constructor receiver/argument origins | P3 | P4 initialization facts, P7 expression | consume SSA bindings, do not maintain a second register-root tracker |
| Constructor delegation targets and known this/super relations | original references, P4 initialization/declaration facts | existing delegation checks and P7/P8 | retain original calls; an unknown chain does not become an output gate for ordinary readable methods |
| Value origins and initialization uses of captures | P0 declarations, P3 value sources, P4 evidence | P5 storage, P7 method expression, P8 ownership | do not bypass missing facts through name prefixes or extra analysis |
| Existing member correspondence evidence such as enum/bridge | original and analysis evidence of the existing stage owners | P8 member presentation | add no hiding rules, no automatic deletion by flags, no re-running members to supply evidence |

This table records known consumption needs and authorises neither wider stage input nor helper
visibility. P8 generates no missing method evidence, and P7 does not call P2/P3 entry builders to
supply constructor origins.
Class context before a method delivers only established required facts and does not depend on a
complete class representation being generated first.

Existing independent entry evidence has an explicit prefix scope and differs from the guarantee of
an ordinary complete method product; migration must retain its concrete semantic checks, must not
treat it as a complete MethodResult, and must not turn it into a second ordinary method recovery
route. Actual scheduling of entry construction is still owned by Engine, and later stages cannot be
given authority to call upstream analysis. If those calls cannot be gathered within the approved
scheduling and permissions, stop the affected migration and keep the existing work and the concrete
conflict instead of widening access or revoking checks.

P8 retains original members by default and does not widen existing Generated rules. Established
correspondence mappings may be consumed as they are, mapped to original member identities and the
syntax they carried; members must not be newly hidden because the whole class is written in
Java-like notation, and other readable members must not be withdrawn because a special syntax was
not recovered.
Without ownership evidence, original names, fields and methods are retained; ownership is not
guessed from `$` or synthetic.

## 9. Local failure and method exit

**Unified principle: an error affects only the scope that actually depends on it; recoverable
fragments in the same method and other ordinary members are preserved, and decompilation is not
made all-or-nothing for a whole method, class or input.**

This section constrains error handling in every stage, shared reads, caches, scheduling, aggregation
and presentation; it is not a special recovery option for P1–P3. §9.1 details the inside of a
method and §9.2 defines chain-wide ownership:

1. The owner of a failed fact MUST limit the impact to the smallest reliably identifiable scope and
   record the original identity, position, cause and invalidated facts. Unknown propagates only
   along real data, control, exception, lock or initialization dependencies; unaffected analysis and
   presentation MUST continue.
2. Error classification and stop scope MUST be separate. The name `Malformed`, `Unsupported`,
   `InternalInvariant` or `ResourceExhausted` is not by itself enough to escalate a method-private
   problem into termination of a whole class, DEX or request. Widening the stop scope MUST name the
   damaged shared fact, the section that cannot be located, or the scheduling, result retention and
   output resources that can no longer be maintained.
3. Missing or unusable local facts MUST be delivered with explicit states, and Unknown must not erase an
   already recorded error. When a local expression cannot be recovered, the method result MUST keep
   the recovered content and mark the unrecovered fragment and the available DEX facts explicitly in
   the corresponding original range; the whole method must not be withdrawn because that fragment
   failed.
4. When instruction boundaries, transfer targets or state associations cannot be determined, the
   owner MUST stop the unreliable range and must not guess the next instruction, skip a bad
   operation or continue using a possibly invalidated state. Other independently locatable
   fragments, members, classes and DEX images that do not depend on that range MUST be retained and
   processed further.
5. Fault tolerance MUST NOT silently delete calls, writes, exceptions, locks or initialization
   effects, nor mask gaps with zero, null, NOP, an empty list, an exception stub or a dispatcher.
   Local tolerance is expressed through explicit fact states and partial results, and a stage
   construction snapshot is not passed off as a complete preceding product.
6. After a method-private resource request fails, the task MUST release scratch it no longer needs
   and retain the existing deliverable facts; as long as basic resources can still sustain
   independent work, other methods MUST continue. When resources are thoroughly unavailable, stop
   the actually affected scope explicitly and promise no recovery through new allocation or rerun.
7. Aggregation MUST combine successful, partially recovered and unavailable results and must not
   withdraw other results because one sub-result failed. A shared cache MUST NOT spread a local
   failure to unrelated objects or cache a read failure as absence. A missing callee analysis result
   is not grounds for the caller's analysis to fail.

A mature success product is delivered only when this stage's guarantees are satisfied. Damaged
required input, unsupported semantic shapes and internal invariant errors keep concrete
classifications and cannot be hidden behind empty lists or default values.

A method task MUST retain the original identity of the failed method, its independently usable
facts, existing recoverable expression and the stage cause. After failure it releases unused
analysis scratch; safe original facts needed by unrecovered fragments are delivered through the
existing one-time read-only exit. The exit does not re-run analysis and does not recover missing
semantics.

Original section/record reads belong to the reader and instruction-decoding facts belong to P1; the
read-only exit reuses the corresponding producers, and P9 renders only existing facts.
Confirmed Malformed is not revoked because presentation is possible. Resource failure retains the
already usable original facts; recovery cannot be promised through new allocation or re-analysis,
and silent successful truncation is not allowed.

Local failure inside a method does not discard the results of other recoverable fragments or
members. Missing optional source metadata is kept separate from failure of required analysis facts,
and existing readable results and concrete uncertainties continue to be delivered. Control flow
that P6 cannot structure naturally is retained as complete explicit graph expression per §5.6 and
does not enter whole-method Debug as an ordinary analysis failure. Missing real execution facts and
internal errors keep concrete classifications and stop the actually damaged scope without
automatically revoking the remaining recoverable expression.

### 9.1 The local-availability contract inside a method

The delivery goal of this section is continued analysis and expression within the same method, not
merely letting the next method continue, and not downgrading a whole method to disassembly at the
first problem. A stage continues with its own bounded algorithm internally; a local state is not an
independent verification process for ordinary requests and does not require a new general error
framework.

#### Product guarantees and publication

- A producer MUST distinguish established facts, Unknown interpretations and Unavailable with the
  original error. Unreadable contents of one original reference do not automatically negate a
  determined instruction width, register or normal transfer. Gaps are recorded per fact item and
  one "whole instruction succeeded" boolean must not erase other facts.
- A mature stage product MUST account completely for source coverage, local states and the impact
  range later processing needs. "Complete" does not require all facts recovered; established items
  still satisfy their original guarantees, and unusable items cannot be consumed as usable input.
  Only the owner can publish a product and must not hand a mutable construction snapshot to a
  successor for repair.
- A producer MUST complete the fact and state associations of one published unit before other units
  refer to it. A local construction error must not leave half an operation, a dangling identity or
  an unfinished Phi as a usable fact.
  When these minimum guarantees cannot be maintained, keep the last trusted product and stop the
  affected construction.

#### Producers and consumers

| Producer | Local information it must deliver | Consumers and handling |
|---|---|---|
| reader / P1 | original identity and readable ranges; each determined boundary; decoded facts, original references and the state of each association; unrecovered sections and concrete causes | P2 converts only established facts and guesses no length, argument layout or missing target; a gap is retained as an unrecovered position in the execution model |
| P2 | operation/terminal identities; facts or states for reads, writes, commits, normal/exception transfers and heap/lock/initialization effects separately; register/control scopes of uncertain impact | P3 builds origins from established relations, keeps origin uncertainty for gap impacts, reads no opcodes and supplies no P2 semantics on its behalf |
| P3 | origin or unavailable state of each definition/use/entry; wide values and overlap impact; gap causes and original positions | P4 solves only from evidenced facts; an unknown interpretation cannot revoke a known SSA origin in reverse |
| P4 / P5 | availability and dependencies of value and use interpretations, storage/copies/snapshots | P6 / P7 continue the planning and expression they can establish; a missing copy, snapshot or initialization cannot be assumed absent by default |
| P6 / P7 | independently recoverable expression and unrecovered fragments with facts and causes associated to original positions | P8 retains the mixed result inside the same method; P9 is pure rendering and retries or redoes no analysis |

Normal paths and failed positions use the same set of original identities, the same operation
definitions and one method chain.
An implementation MUST NOT build a separate opcode interpreter, region-rerun entry or second
register tracker for gaps.
The actual Rust type layout is decided by the owning module; this table widens neither construction
authority nor upstream input access.

#### Gap impact and value sources

1. P2 MUST distinguish known reads, known normal writes and a register range that is merely
   possibly affected.
   An unknown read or write is not an empty one; when an uncertain range cannot be narrowed, widen
   it explicitly to the possibly affected range and do not force an unrelated declaration for local
   appearance. Boundary computation does not pre-expand a dense relation matrix.
2. P3 MUST distinguish "which definition the value comes from" from "how that value is
   interpreted". When a target name, constant content or type interpretation is unknown but the
   definition, slot and commit relations are established, the SSA origin can still be retained.
   Only when origin evidence is missing does the origin stay unestablishable, and no ValueId or
   default register initial value may be fabricated.
3. A write or commit gap affects only tracked slots whose possible write range intersects it and the
   origin inferences that genuinely depend on them. Known disjoint slots are unaffected. When
   control relations are known, one definite later write can truncate the register's uncertainty,
   but it must not clear heap, lock or exception uncertainty along with it, nor repair an unknown
   entry that may still bypass that write.
4. P2 MUST retain the origin, known roles and possible entering range of an undetermined transfer.
   P3 MUST NOT delete that transfer and then compute trusted dominance or Phis from a projection of
   known edges as if it were the complete CFG. The corresponding binding may be delivered only when
   unknown extra entries can be excluded or their origin uncertainty is explicitly included.
5. P3 MUST keep normal and exceptional states separate. When commit rules are unknown, "no write",
   keeping the old value or treating the case as the normal path must not be assumed; when the rules
   are established, the exception frame before the normal commit is still retained.
   Missing wide-value or incoming evidence keeps a local unavailability; wide values must not be
   spliced and bad sources must not be ignored.
6. When an undetermined range may affect the whole method, the producer MUST widen the uncertain
   range truthfully; arbitrary errors do not promise to affect only one instruction. Even when
   analysis cannot continue, independently usable original facts are retained.

#### Presentation, failure and acceptance

P7 MUST retain, inside the same method result, the recoverable expression together with the original
ranges, concrete causes and available DEX facts of unrecovered fragments. A gap is not a fake runtime
operation: no padding call, default value, empty statement or exception stub may be generated to mask
unknown behaviour. Unnatural graph shape is still expressed per §5.6; missing execution facts cannot
be declared recovered by substituting a goto. P9 identifies no region, derives no type and re-runs no
stage.

When a stage's minimum guarantee cannot be maintained, a basic structure cannot be located safely or
results can no longer be recorded, stopping the actually affected scope is permitted. The stop MUST
retain independently published facts and expression already delivered. If only original fragments
can be delivered, they must be presented truthfully and not counted as existing Java-like recovery.

Acceptance: through the existing approved public Engine core tests and formal observation entries,
independently check that:

- A local reference problem in one method does not prevent its later independent operations from
  entering P2/P3; the original error still exists.
- A local possible-write gap affects only the dependent slots; with known control relations, origin
  recovers after a definite rewrite.
- An unknown transfer or exceptional commit does not produce false dominance, Phis or readable
  values by deleting edges.
- When a width cannot be determined, later boundaries are not guessed; a stage construction failure
  publishes no dangling identities or falsely complete product.
- Existing behaviours for old exceptional state, wide-value overlap, argument order and effect
  retention do not regress.

These are target contracts and acceptance conditions, not a claim that the current implementation
passes. Current capabilities are documented in the
[engine README](../../crates/diana-decomp/README.md) and active source.

### 9.2 P1–P9 uniformly consume local results

Every P1–P9 owner MUST use the product guarantees of §4 and the impact-scope rules of §9.
This contract runs through analysis, storage, control, expression, assembly and rendering; it is not a
special recovery mechanism for P1–P3, and Engine cannot replace it by wrapping an outer catch.
Peripheral input management does not replace this work.

| Stage | Consumption obligations on input | Local result of this stage and the next stage's guarantee |
|---|---|---|
| P1 | consume the safe ranges and independent read states already delivered by the reader, rebuilding no source | retain decoding, reference, operand and association states per item; P2 can distinguish decoded items, known-range gaps and sections with unknown boundaries |
| P2 | establish execution relations over established P1 facts, guessing no missing declaration, width or target | separate availability of reads, writes, commits, normal/exception transfers and effects and deliver gap impact ranges; P3 need not re-decode or delete edges to complete the graph |
| P3 | use only established P2 execution relations and undetermined impacts, treating no projected graph as a complete CFG | every definition, use and incoming retains its origin or local unavailability; P4 obtains accurate widths, availability and original causes and does not confuse an unknown interpretation with a lost origin |
| P4 | solve from each origin and original constraint, fabricating no type and repairing no SSA in reverse | ValueFacts, UseViews, PhiViews and initialization interpretations each deliver their result and impact; P5 can continue independent storage without one failed use revoking the method's type results |
| P5 | establish storage from well-evidenced representations, adaptations and positions, supplying no defaults for unknown origins | storage bindings, copies, snapshots and initialization uses each retain a plan or a gap; P6/P7 can identify affected uses and paths and cannot treat a missing plan as needing no execution |
| P6 | consume the local states of storage and execution facts, supplying no missing copy, target or handler | independent regions continue planning and known unnatural control uses GraphRegion; real gaps keep their impact range, and P7 need neither re-identify structure nor assume unknown transfers were recovered |
| P7 | express only existing semantics and plans and must not re-run prior analysis because input is partly usable | the same MethodResult retains recovered expression and unrecovered fragments with original ranges, causes and facts; a local AST failure does not blank other body content, and P8 needs no second recovery |
| P8 | combine members and intra-method fragments by original identity without requiring every member or expression to succeed | ClassResult/TypeNamePlan retain mixed results and local name/ownership problems; P9 obtains existing display facts and gap states and discards no code because of name-planning problems |
| P9 | purely render existing expression, display plans and unrecovered facts, performing no analysis | text/JSON retain the same readable fragments, original identities and causes; a local unavailability is not an output gate for the whole result, and a real output failure does not masquerade as successful truncation |

Each consumer MUST handle the state of the facts it uses rather than re-verifying the whole upstream
product.
Each producer MUST complete local state and origin associations inside its own private construction
and must not leave a half-finished product for the next stage to repair. A successor may widen the
actual dependency impact, but MUST NOT lose the original cause or unconditionally fold an upstream
local state into whole-method failure.

A stage algorithm continues its own bounded processing over established facts and delivers concrete
states for insufficiently evidenced parts. The owner stops the corresponding construction only when
it cannot maintain the minimum guarantees of identity, coverage or required relations; no separate
region rerun, recovery branch or general error manager is required per error. Safe parsing,
range/arithmetic checks, allocation failure and backpressure handling remain, and independent
verification still belongs only to tests.

Acceptance MUST check recovered content, explicit gaps and coverage separately and cannot substitute
a whole-result boolean. P1–P3 check continued fact processing per §9.1; P4–P6 check that one use, copy
or region problem does not revoke other established results; P7–P9 check that the same method's
recovered fragments actually reach the final text and JSON and that gaps retain original positions
and causes. Keeping only disassembly, or letting only the next method continue, is insufficient for
end-to-end acceptance of intra-method recovery.

Acceptance follows the existing authorisation for tests and authorises no new test suite, private
stage harness, non-core tool test or CI workflow. Process termination, an unlocatable shared input or
a thorough loss of basic resources promise no continuation; these situations also cannot become the
default reason for a global stop on an ordinary local error.

## 10. Implementation strategy: new structures first

### 10.1 The current implementation is only a semantic and algorithmic reference

The current P0 is a standalone ClassSnapshot that is not connected to formal method scheduling. The
existing method flow still has fixed entry Phis, cross-stage raw queries and checks, and whole-method
structure templates/dispatchers.
These layouts and call patterns are not compatibility targets for the new implementation; each
stage's product contract is the basis for implementation.

Implementers MAY reimplement algorithms, products and modules from the existing code and are not
required to move files one by one or keep old IR fields, internal helpers, nesting and call
signatures. Implementation first forms the actual producer/consumer relations of the new structures
and does not wrap the old route in forwarding wrappers to look like new products.

Shared safe reads, format/operand tables and local algorithms that genuinely fit the new contracts
may be reused. Unsuited implementations may be rewritten; reuse is decided by input guarantees,
semantic invariants and consumption needs, not by relaxing new interfaces to keep old modules
compiling. The shared original-format implementation stays single, and rewriting must not create a
second opcode width table or method-decoding/recovery route.

The value of the old code is in identifying original facts, semantic conditions, classified failures
and check obligations. A rewrite may change the implementation form of a check but cannot delete its
safety/semantic obligations without reason. New checks are completed inside the owner of the
corresponding transformation; original replay is not all moved to P1, and stage trust is not built by
duplicating upstream algorithms.

### 10.2 Implement along the new product chain

1. Implement the new P0 ClassFacts, semantic flags, four-group member directory and bound method
   entries, separating directory coverage, declaration and metadata states. Do not use current debug
   JSON fields as the internal product model.
2. Implement the shared method reader's MethodSource and P1 DecodedMethod. Make the distinction
   between original section reads and disassembly/instruction association explicit, and have
   consumers neither rescan directories nor decode instructions.
3. Implement P2's complete FlowMethod, closing operation semantics, commits, transfer identities and
   origin coverage inside P2 without depending on old P3 fields or helpers.
4. Implement P3's exception-aware pruned SSA, dominator tree, on-demand IDF and renaming. Use the
   state model and private position mapping of §5.3 and drop the old assumption that a cross-block use
   must pass through an entry Phi.
5. Implement P4's separated constraint domains, ValueFacts/UseViews/PhiViews and initialization facts,
   consuming original signatures directly and never making natural-expression conditions an analysis
   admission requirement.
6. Implement P5's storage mapping, dependency index/ready queue, copy cycle breaking and exception
   snapshots, consuming complete incoming and StatePoint obligations without the old LocalId=ValueId
   layout convention.
7. Implement P6's exit-directed regions and GraphRegion, with normal structure first and Java-like
   transfers preserved when structuring fails. Implement P7's unified operation expression, natural
   expression for ordinary calls/new, and full pseudo-code capability, without producing a dispatcher
   or withdrawing a whole method for insufficient expression.
8. Implement P8's all-member class assembly, original static preparation and TypeNamePlan, plus P9's
   pure rendering. Name arrangement does not replace text inside finished strings, method results are
   not re-run, and final spans are computed from actual output.
9. Connect the new products to Engine's approved method-level scheduling and application result
   delivery, delete old production/analysis/rendering paths that are actually superseded, and update
   stage documents and module declarations with them.

Complete real consumers product by product instead of accumulating module skeletons without
consumers first. A connection unit must cover the relevant original identities, local failure,
resource and check obligations together, not merely the success path.

Engine's ownership, ordered calls, scoped method concurrency and class-level joining keep the approved
relations.
A rewrite does not authorise new stage entries or wider exports or construction authority. Existing
application queries and input-safety contracts are retained, and building a long-term compatibility
layer on old private IR is not required.

Respect existing uncommitted work while preparing the new implementation; do not withdraw unrelated
changes to clean up structure. After the new product chain enters the same formal entry, two ordinary
method pipelines cannot be kept long term, and switching back to the old algorithm by input name,
failure or output appearance is forbidden. A rewrite is not an extra chance to decompile again.

### 10.3 Old-code comparison checklist: keep the obligations, do not move files

| Old-code reference location | Obligation in the new structure |
|---|---|
| p1_input/catalog, field_declarations, annotation_facts | P0 class facts and shared reader: member/initial-value association, local failure, usable prefixes and single fact production |
| p1_input/code, method, structure, formats/operands | MethodSource/P1: range, instruction/reference/boundary and metadata guarantees; the shared format table stays single |
| p2_flow/normal, local_ops, protection | P2: accurate reads/writes/effects, producer/result, normal/exception transfers and origin coverage |
| p3_ssa/normal, word_ownership | P3: value origins, wide values/invalidation, entry back-edges and old exceptional state; the construction algorithm is replaced by the new plan |
| p4_types/normal, numeric, zero, initialization, declarations | P4: type/use interpretation, original references, Unknown, initialization state and bounded solving |
| p5_variables/copies, exceptional, object_allocation | P5: simultaneous assignment, temporaries, site snapshots and materialization without re-deriving origins/types |
| p6_regions region, protection and lock handling | P6: complete region/transfer handling and protection/lock correspondence without rewriting the original CFG |
| p7_java operation expression, constructors, class_unit and static preparation | P7 method expression and P8 class assembly: original identities, execution order, all-member results, without inheriting a Java-eligibility gate |
| p8_emit and application result presentation | P9: rendering of existing products, concrete diagnostics, original origins, output control and the same result for text/JSON |
| engine/member_emission, constructor_chain evidence orchestration | retain the known evidence obligations per §8.4, adding neither member hiding, entry reruns nor cross-stage construction authority |

This table is not a migration task list and does not require a new file for every old file. Existing
logic may be deleted, merged or rewritten independently, as long as the obligation is implemented
explicitly inside the new owner and the superseded path is removed.
When a fact is missing, fix its production contract rather than pushing the problem downstream under
the name of a rewrite.

### 10.4 Completion conditions and pre-implementation checks

The completion condition is that the new product chain actually serves formal single-class and
single-method requests, stages consume only approved product interfaces, original identities,
available content and check obligations have explicit owners, and superseded routes are removed.
Directory renames, old-IR compatibility or successful compilation of new types do not constitute a
completed refactoring.

Acceptance: original member and instruction/argument/effect order, numeric bits,
exception/lock/initialization behaviour, local failure, available output, and resource and
backpressure behaviour are preserved per the product contracts. Follow the unit-test prohibition in
the root AGENTS.md; formatting, Clippy and builds cannot prove semantic correctness or representative
input coverage. Performance conclusions come only from release measurements of the formal
single-class flow on authorised inputs.

Sections 4.4–4.5, 5.3.4–5.3.6, 5.4.5, 7.1–7.2 and 8.3–8.4 already state the contracts for product
ownership, state, exceptions/wide values, solving domains, declarations and class assembly. They are
design obligations, not proofs of implementation or general correctness. Concrete private fields,
indexes and module layouts are decided by the implementer before implementing that product and do not
require item-by-item re-approval.

Implementers MUST check the input/output, semantic interpretation, commit positions, transfers and
original origins of each operation category, including in particular overlapping invalidation,
classification of conservative paths, site protection and complete expression coverage.
Known class-level evidence must also be checked for its real upstream consumption needs, and
scheduling must not be changed additionally to close a gap.

The four prohibited constraint groups of the root AGENTS.md continue to apply: do not change agreed
output, hide members or add a dispatcher; do not introduce external declarations, new dependencies or
default numeric limits; do not additionally change stage ownership or scheduling; do not widen stage
input access, exports or construction authority. On a real conflict, stop the affected implementation
and report the cause instead of adding a bypass, relaxing a guarantee or quietly changing the plan.
