# Entwurf: Register-Interface für die Färbung

Status: Entwurf, noch nicht umgesetzt. Baut auf dem Interferenzgraphen in `zerok/src/spill/` auf.

## Ziel

Die Färbung (Simplify, Select, Spilling) soll für jede Architektur funktionieren und nur mit Farben
`0..K` rechnen. Jede Architektur liefert die Zuordnung zwischen Farben und echten Registern. Knoten
einer Registerklasse dürfen dabei nie eine Farbe einer anderen Klasse bekommen. Das soll schon der
Compiler prüfen.

## Ausgangslage im Code

- Der Graph (`IntfGraph = Cache<RegNum, IntfEntries, IntfEdge>`) hat einen Knotentyp pro
  Registerklasse: `IntNode` und `FloatNode`.
- Zwischen Registern verschiedener Klassen gibt es keine Kanten.
- Jeder Knoten hat eine `NodeColor`: `Uncolored`, `Precolored(Color)`, `Colored(Color)` oder
  `Spilled`.
- Die Klasse kommt bisher aus `RegClass::of(kind)`: `F32`/`F64` → `Float`, `None` → kein
  Register, alles andere → `Int`.

## 1. Eine Registerdatei pro Klasse

```rust
/// Die Register einer Klasse (z.B. die allgemeinen Register oder die xmm-Register)
pub trait RegFile
{
    type Reg: Copy + Eq + Debug + 'static;

    /// Allokierbare Register. Index = Farbe, Reihenfolge = Präferenz beim Färben
    const ALLOCATABLE: &'static [Self::Reg];
    const K: usize = Self::ALLOCATABLE.len();

    /// Register, die ein Call überschreiben darf
    const CALLER_SAVED: &'static [Self::Reg];

    /// Register für den Rückgabewert dieser Klasse
    const RETURN: Self::Reg;

    fn reg_of(c: Color) -> Self::Reg { Self::ALLOCATABLE[c] }
    fn color_of(r: Self::Reg) -> Option<Color> { Self::ALLOCATABLE.iter().position(|&x| x == r) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysV64Gpr { Rax, Rcx, Rdx, Rsi, Rdi, R8, R9, R10, Rbx, R12, R13, R14, R15 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysV64Xmm { Xmm0, Xmm1, /* … */ Xmm15 }

impl RegFile for SysV64Gpr { type Reg = Self; /* … */ }
impl RegFile for SysV64Xmm { type Reg = Self; /* … */ }
```

`K` ist die Zahl der **vergebbaren** Register, nicht aller Register. Nicht in `ALLOCATABLE`
gehören:

- `rsp`, der Stack-Pointer
- `rbp`, falls ein Frame-Pointer verwendet wird
- mindestens ein Scratch-Register für den Spill-Code

Die Reihenfolge in `ALLOCATABLE` bestimmt, welche Farbe Select zuerst versucht.

## 2. Die Architektur fasst die Dateien zusammen

```rust
pub trait Arch
{
    type Int: RegFile;
    type Float: RegFile;

    /// In welcher Registerdatei ein Wert von `kind` liegt (None = braucht kein Register)
    fn class_of(kind: Kind) -> Option<RegClass>;

    /// Wo die Parameter mit diesen Klassen beim Aufruf liegen
    fn arg_locations(classes: &[RegClass]) -> Vec<ArgLoc>;
}

pub enum ArgLoc
{
    Reg(RegClass, Color),
    Stack(usize),
}

pub struct SysV64;

impl Arch for SysV64
{
    type Int = SysV64Gpr;
    type Float = SysV64Xmm;
    // …
}
```

Die Architektur wird zur Laufzeit gewählt, etwa per CLI-Flag. Das passiert an genau einer Stelle:

```rust
pub enum Target { SysV64, /* Win64, AArch64, … */ }

match target
{
    Target::SysV64 => allocate::<SysV64>(&mut graph),
}
```

## 3. Der Knotentyp wählt seine Registerdatei

Ein generischer assoziierter Typ (GAT) an `IntfNode` legt fest, aus welcher Datei ein Knotentyp
gefärbt wird:

```rust
pub trait IntfNode: CacheEntry<IntfEntries> + IdentRef<RegNum>
{
    const CLASS: RegClass;

    /// Die Registerdatei dieser Klasse auf der Architektur `A`
    type File<A: Arch>: RegFile;

    // new, color, set_color wie bisher
}

impl IntfNode for IntNode   { type File<A: Arch> = A::Int;   /* … */ }
impl IntfNode for FloatNode { type File<A: Arch> = A::Float; /* … */ }
```

Die Färbung ist generisch über Architektur und Knotentyp. `K` und die Rückübersetzung in Register
laufen ausschließlich über `N`:

```rust
fn color<A: Arch, N: IntfNode>(graph: &mut IntfGraph)
{
    let k = <N::File<A> as RegFile>::K;
    // Simplify/Select auf graph.variants::<N>() und neighbours::<N>
}

color::<SysV64, IntNode>(&mut graph);
color::<SysV64, FloatNode>(&mut graph);

// Im Backend:
let reg = <<IntNode as IntfNode>::File<SysV64> as RegFile>::reg_of(c); // SysV64Gpr
```

Knotentyp, `K`, Nachbarn und `reg_of` hängen alle am selben `N`. Knoten einer Klasse können
deshalb nicht mit den Farben einer anderen gefärbt werden. Eine neue Klasse braucht einen neuen
Knotentyp und einen neuen assoziierten Typ in `Arch`. Fehlt eines davon, kompiliert der Code nicht.

### Was nicht in den Typen steckt: die Farbe

`NodeColor` speichert weiter ein einfaches `Color`. Ganz typsicher würde es erst mit einem
`Color<F: RegFile>` im Knoten. Dann wäre aber der ganze Graph generisch über die Architektur.
Eine Farbe ist ohnehin nur ein Index **innerhalb** der Datei ihres Knotens. Solange sie nur über
`N::File<A>` in ein Register übersetzt wird, landet sie nicht in der falschen Datei.

Empfehlung: `Color` als Newtype `struct Color(usize)` statt als Alias. Dann lassen sich Farben
nicht mit `RegNum` oder `CmdIdx` verwechseln, die alle `usize` sind.

## 4. Aufrufkonvention

`RETURN` und `CALLER_SAVED` sind Eigenschaften einer Datei. Die Zuordnung der Parameter dagegen
nicht, weil die Architekturen unterschiedlich zählen. Beispiel `f(a: i64, b: f64, c: i64)`:

| | `a` | `b` | `c` |
|---|---|---|---|
| SysV64 (jede Klasse zählt für sich) | `rdi` | `xmm0` | `rsi` |
| Win64 (Position zählt über alle Klassen) | `rcx` | `xmm1` | `r8` |

Deshalb sitzt `arg_locations` an `Arch` und nicht an `RegFile`.

Daraus leitet ein generischer Durchlauf über den IR die Vorfärbung ab:

| IR | Bedingung |
|---|---|
| `Func { args }` | `args[i]` liegt an `A::arg_locations(..)[i]` |
| `Call { args, reg }` | Argumente an `A::arg_locations(..)`, Ergebnis in `RETURN` der Klasse von `reg` |
| `Ret` bzw. `Reserved::Return` | `RETURN` der Klasse |
| jedes Register in `out[n] \ def[n]` an einem `Call` | interferiert mit allen `CALLER_SAVED` seiner Klasse |

Die Zuweisungen der Parameter werden als Move-Kanten zu den Maschinenregistern umgesetzt, nicht
als Interferenz. Eine gute Färbung lässt den Move dann ganz wegfallen.

Befehle mit festen Registern (x86 `idiv` → `rax`/`rdx`, Shift → `cl`) kommen später über eine
zusätzliche Methode mit leerer Default-Implementierung dazu.

## 5. Architekturen mit geteilten oder anderen Registerdateien

### Floats in den Int-Registern (Soft-Float)

Beispiele: RISC-V ohne F-Erweiterung, ARM soft-float, AVR.

Mit `RegClass::of(kind)` wäre das ein Fehler. Zwischen `IntNode` und `FloatNode` gibt es keine
Kanten, beide würden aber aus derselben Datei gefärbt. Ein `i64` und ein `f64`, die gleichzeitig
leben, könnten dasselbe Register bekommen.

Lösung: Die Architektur entscheidet über die Klasse (`Arch::class_of`), nicht der Typ des Werts.

```rust
// x86-64:     F32 | F64 => Some(Float),  None => None,  _ => Some(Int)
// Soft-Float: None => None,  _ => Some(Int)
```

Ein Knotentyp steht damit für eine **Registerdatei**, nicht für eine Art von Wert. Auf
Soft-Float werden Floats zu `IntNode`s, interferieren korrekt mit den Ints, und es gibt keinen
einzigen `FloatNode`. Treffendere Namen wären `GprNode` und `FprNode`.

Folge für den Code: `register_classes` und `build_interference_graph` werden generisch über
`A: Arch`. Die Liveness bleibt unabhängig von der Architektur.

### Andere oder zusätzliche Klassen

| Situation | Beispiel | Umgang |
|---|---|---|
| gleiche Aufteilung wie x86 | AArch64 (`x0–x30`, `v0–v31`), RISC-V mit F | passt direkt in `Int`/`Float` |
| zusätzliche getrennte Datei | Vektorregister (RISC-V V), Masken (AVX-512 `k0–k7`, SVE `p0–p15`) | neuer Knotentyp + assoziierter Typ in `Arch` |
| Unterregister derselben Datei | x86 `al`/`ax`/`eax`/`rax` | kein Problem, alles liegt in der GPR-Datei |
| ein Wert braucht zwei Register | `i64` auf 32 Bit | vor der Allokation aufspalten |
| überlappende Register verschiedener Breite | ARM32 VFP: `d0` = `s0` + `s1` | vermeiden (siehe unten) |
| Befehle wollen bestimmte Register | `idiv`, Shift | Vorfärbung |

Registerpaare und Überlappungen brechen die Regel „Grad < K heißt sicher färbbar“, weil ein Knoten
mehr als eine Farbe belegt. Statt den Allokator dafür zu erweitern, werden diese Fälle **vor** der
Allokation beseitigt:

1. **Legalisierung:** Ein Durchlauf vor der Liveness spaltet Werte auf, die nicht in ein Register
   passen (`i64` auf 32 Bit wird zu zwei 32-Bit-Registern). Soft-Float-Rechnungen werden hier zu
   Aufrufen von Bibliotheksfunktionen.
2. **Eingeschränkte Registermengen:** Bei überlappenden Dateien nur eine Breite in `ALLOCATABLE`
   aufnehmen, zum Beispiel auf ARM32 nur die `d`-Register.
3. **Vorfärbung** für Befehle mit festen Registern.

Der Allokator sieht dann nur gleichförmige Klassen.

## Offene Fragen

- ~~**Schlüssel der Maschinenregister im Graphen**~~ **Entschieden:** Der Graph verwendet
  `enum NodeId { Virt(RegNum), Phys(RegClass, Color) }` als Schlüssel. Die Klasse gehört mit in
  den Schlüssel, weil eine Farbe allein mehrdeutig ist (`Color(0)` ist `rax` und `xmm0`). Knoten
  für Maschinenregister entstehen nur über `N::phys(color)`, das die Klasse aus `N::CLASS` nimmt
  und den Knoten mit seiner eigenen Farbe vorfärbt. Der Bereich `0..256` wird dafür nicht
  gebraucht.
- **`Reserved::Return`:** Das ist im Grunde schon ein vorgefärbter Knoten (`RETURN` der Klasse)
  und könnte mit der Vorfärbung als Sonderfall entfallen.

## Reihenfolge der Umsetzung

1. `RegFile`, `Arch` und `SysV64` mit `SysV64Gpr`/`SysV64Xmm` anlegen.
2. `IntfNode::File<A>` ergänzen.
3. `Arch::class_of` statt `RegClass::of` und den Graphaufbau generisch über `A` machen.
4. ~~Schlüssel der Maschinenregister klären~~ (`NodeId`) und die Vorfärbung umsetzen.
5. Simplify und Select als `color::<A, N>`.
6. Spill-Code erzeugen und die Schleife bis zur gültigen Färbung schließen.
