# Problem

Moto Devil hat drei Levels, die alle gleich aussehen: blauer Himmel, brauner
Boden, graue Felsen. Wer eins geschafft hat, merkt beim nächsten nicht, dass er
irgendwo Neues angekommen ist. Es fehlt ein Gefühl von Reise und ein sichtbares
Ziel, auf das man hinarbeitet („Ich bin schon in der Schweiz!“).

Gleichzeitig wird die Reihenfolge der Levels zufällig schwieriger oder leichter.
Das Testlevel „Fallen & Flugstunden“ führt viele Mechaniken einzeln ein, ist aber
nicht als Steigerung nach Level 2 gebaut. Wenn mehr Levels dazukommen, braucht es
eine Regel, damit jedes Level etwas mehr verlangt als das vorherige, ohne unfair
zu werden.

Und alle Fallen sind überall gleich. Eine Lawine gehört in die Berge, Treibsand
in die Wüste, ein Känguru ins Outback. Fallen, die zur Landschaft passen, erklären
sich zum Teil von selbst und machen jedes Land unverwechselbar.

Vor allem soll Moto Devil seinem Namen gerecht werden: Die Levels sollen einen
**ständig verarschen**, nicht nur an ein, zwei Stellen.

# Lösung

**Levels werden in Länder gruppiert. Jedes Land hat einen eigenen Hintergrund,
eine typische Landschaft und eigene Fallen; innerhalb eines Landes wird jedes
Level schwieriger als das vorherige, und jedes Level trollt vom Start bis zum
Ziel.** Man fährt die Länder der Reihe nach, wie heute die Levels.

## Die Länder

| Land | Levels | Landschaft | Hintergrund | Landestypisch |
| --- | --- | --- | --- | --- |
| **Übungsgelände** | Level 1, Level 2, Fallen & Flugstunden (die bestehenden, unverändert) | wie bisher: Hügel, Rampen, Looping | grüne Hügel mit Bäumen und Wolken | keine eigene Falle; hier lernt man die gemeinsamen Fallen |
| **Ägypten** | Dünen, Tal der Könige, Die Pyramide | Dünenwellen, Grabkammern mit niedriger Decke, Pyramidenterrassen | Sonne, Pyramiden, Dünen und Palmen | **Treibsand**, Felsbrocken |
| **Australien** | Outback, Schluchten, Am roten Felsen | rote Ebene, Schluchten, ein grosser roter Fels | roter Monolith, Eukalyptusbäume, Büsche | **Kängurus** (hüpfende Kugeln) |
| **USA** | Wüste, Canyon, Goldmine | Wüste, Rampen über Canyons, Minenstollen | Tafelberge, Saguaros, grosse Sonne | **Saguaros** (Kaktus), **Steppenläufer** (rollende und springende Kugeln) |
| **Paris** | Über den Dächern, Montmartre, Eiffelturm | Fahrt über die Dächer der Skyline; Lücken zwischen Häusern fallen auf die Strasse | Eiffelturm, Dächer mit Kaminen, Häuserzeilen | **Tauben** (wie Raben), **Fässer**, Dachluken |
| **Schweiz** | Alpweg, Pass, Gipfel | steile, lange Anstiege und Abfahrten, Lawinengalerien, Gletscherspalten | Bergketten mit Schneekappen, Tannen, Alphütte | **Schneelawine**, Schneebälle |

Die Reihenfolge ist Übungsgelände, Ägypten, Australien, USA, Paris, Schweiz.
Flache Dünen sind leichter zu fahren als Dachlücken oder steile Berge; die
Schweiz mit ihren Lawinen bildet den Schluss.

Die bestehenden drei Levels behalten ihre IDs und ihre Reihenfolge; sie bilden
das erste Land. Damit verschiebt sich kein gespeicherter Fortschritt.

## Wie man ein Land erlebt

- **Ankommen.** Zu Beginn jedes Levels erscheinen für zwei Sekunden Land, Nummer
  und Name, zum Beispiel „Schweiz“ und darunter „2/3 · Pass“. Das HUD zeigt
  dauerhaft „Schweiz 2/3 · Pass“.
- **Abschliessen.** Nach dem letzten Level eines Landes sagt der Zielbildschirm
  „Ägypten geschafft!“ und „Enter: weiter nach Australien“. Nach dem allerletzten
  Level: „Alle Länder geschafft!“.
- **Spielerauswahl.** Der Fortschritt nennt das Land, in dem der Spieler gerade
  steckt, zum Beispiel „Paris · 13 von 18 Leveln geschafft“. Wer alles geschafft
  hat, bekommt „alle Länder geschafft ✓“.
- **Grabsteine** bleiben pro Level, wie bisher.

Es gibt keine Weltkarte und keine Länderauswahl; die Reihenfolge ist fest.

## Ständig verarscht

Jedes Länder-Level reiht Lügen aneinander, schon ab dem ersten Meter:

- Ein Block schiesst direkt vor dem Start aus dem Boden, oder ein Block stürzt aus
  dem leeren Himmel (er hängt ausserhalb des Bildes).
- Landeplätze, Dachluken und Brücken brechen weg.
- Stacheln verstecken sich in harmlosem Sand, auf Dünen und vor dem Ziel.
- Die Zielflagge läuft weg, nach vorn über eine Lücke, zurück hinter einen, oder
  zweimal hintereinander.
- Hilfen werden zur Falle: Ein Trampolin wirft einen zurück, ein Jet-Flug führt
  an einer Taube vorbei.
- Landesfallen kommen dazu, wenn man es am wenigsten erwartet: Treibsand direkt vor
  einer Rampe, eine Lawine, die einen auf eine wegbrechende Brücke treibt.

Jede Lüge hat genau eine verständliche Antwort: springen, warten, unterstellen,
schnell bleiben oder umdrehen. Beim zweiten Versuch ist jede Stelle fair.

## Hintergrund

Der Hintergrund ist Kulisse: keine Kollision, keine Gefahr, kein Hinweis auf
Fallen. Er besteht wie der Rest des Spiels aus einfachen farbigen Formen.

- **Zwei Ebenen.** Eine ferne Ebene (Berge, Pyramiden, Tafelberge, Eiffelturm,
  Sonne, Wolken) und eine nähere (Hügel, Dünen, Palmen, Saguaros, Dächer, Tannen,
  Hütte). Beide bewegen sich langsamer als die Strecke, die ferne am langsamsten.
- **Himmelsfarbe** und Farben von Boden und Oberfläche wechseln mit dem Land:
  Gras und Fels in der Schweiz, Schnee auf hohen Streckenteilen; Sand in Ägypten;
  rote Erde in Australien; in Paris Hausfassaden mit Fenstern und Zinkdächern.
- **Lesbarkeit geht vor.** Kulissenfarben sind gedämpft und verblassen zum Himmel
  hin; die Saguaros im Hintergrund sind blasser als der klebrige Kaktus auf der
  Strecke. Gefahrenfarben kommen im Hintergrund nicht vor.
- **Bekannte Formen, keine Marken.** Eine Bergsilhouette darf an das Matterhorn
  erinnern, Pyramiden an Gizeh, der Turm an den Eiffelturm. Keine Logos, Firmen-
  oder Produktnamen und keine Figuren von Menschen.

## Landesfallen

Beide neuen Fallen folgen den Spielregeln aus dem
[Fallen-Pitch](../fallen-powerups/pitch--fallen-powerups.md#gemeinsame-spielregeln):
fester Auslöser, gleicher Ablauf bei jedem Versuch, ein lesbarer Hinweis, eine
mögliche Antwort, sauberes Zurücksetzen.

| Falle | Was passiert? | Was kann der Spieler tun? |
| --- | --- | --- |
| **Schneelawine** | Oben am Hang hängt ein Schneebrett mit einem Riss. Fährt das Bike in die Auslösezone, zittert es kurz, rutscht dann zur Strecke hinunter und rollt mit fester Geschwindigkeit weiter, auch über das Dach einer Lawinengalerie. Am Ende ihrer Bahn löst sie sich auf. Berührt sie Bike oder Fahrer, crasht man. | **Von vorne:** sich unter dem Dach einer Galerie unterstellen und warten, bis sie darüber gerollt ist. **Von hinten:** Vollgas davonfahren; ist sie zu schnell, rechtzeitig unter einer Galerie anhalten. |
| **Treibsand** | Heller Sand mit kleinen Strudeln. Solange ein Rad ihn berührt, sinkt das Bike ein; im Stand schneller als in voller Fahrt. Gas und Höchsttempo sinken, der Sand bremst. Sand steigt sichtbar am Bike hoch; ist es ganz versunken, crasht man. | **Nicht anhalten:** Mit Tempo kommt man durch kurzen Treibsand. In langem springt man heraus; draussen taucht das Bike schnell wieder auf. Das ist das Gegenteil von Matsch, wo langsames Durchfahren sicher ist. |

Bestehende Fallen sehen in jedem Land anders aus, ohne anders zu wirken: Hüpfende
Kugeln werden in Australien zu Kängurus, rollende Kugeln in den USA zu
Steppenläufern, in Paris zu Fässern und in der Schweiz zu Schneebällen. Raben sind
in Paris Tauben. Der Kaktus steht als Saguaro in der Wüste der USA. Trefferbereich
und Ablauf bleiben gleich.

## Schwierigkeit von Level zu Level

Innerhalb eines Landes verlangt jedes Level mehr als das vorherige. Das erste
Level eines Landes führt die Landesfalle allein und mit Platz ein; das dritte
kombiniert sie mit bekannten Fallen und hat nur noch einen Checkpoint. Über die
Länder hinweg werden Geometrie und Kombinationen anspruchsvoller: von flachen
Dünen über Schluchten und Canyons und Dachlücken bis zu steilen Bergen mit
Lawinen.

Schwieriger heisst, an einem oder mehreren dieser Regler zu drehen:

- **Dichte:** mehr Fallen pro Streckenstück, weniger ruhige Strecke dazwischen.
- **Kombinationen:** bekannte Fallen treten zusammen oder direkt nacheinander auf.
- **Fahrkönnen:** breitere Lücken, steilere Hänge, Sprünge über Rampen.
- **Checkpoints:** längere Abschnitte zwischen zwei Checkpoints.
- **Lügen:** mehr Stellen, die harmlos aussehen und einen reinlegen.

Keine Regler sind: unsichtbare Gefahren ohne Hinweis beim zweiten Versuch, Zufall,
schwierigere Steuerung, Zeitlimits oder Fallen ohne mögliche Antwort.

## Ein spielbarer Umfang

**Ägypten**

1. **Dünen.** Ein Block schiesst am Start aus dem Boden, kurzer Treibsand, Stacheln
   im Sand, eine Grube, langer Treibsand, ein Rabe über der Düne, ein Block aus
   dem Himmel, und das Ziel hüpft weiter hinter Treibsand.
2. **Tal der Könige.** Ein Felsbrocken, eine Grabkammer, in der Springen den Kopf
   kostet und ein Block von der Decke fällt, Treibsand im Grab, ein Boden, der über
   Treibsand wegklappt, ein Block aus dem Boden, TNT, Stacheln und ein Ziel, das
   zurückspringt.
3. **Die Pyramide.** Stacheln am Start, Treibsand vor einer Rampe über eine Grube,
   ein Block aus dem Himmel und ein Felsbrocken auf der ersten Terrasse, Stacheln
   und eine versteckte Grube auf der zweiten, und das Ziel auf der Spitze springt
   zurück auf die Terrasse.

**Australien**

1. **Outback.** Ein Block aus dem Himmel am Start, ein Känguru, roter Matsch vor
   einer Schlucht, ein Felsbrocken, ein Rabe, ein Block aus dem Boden und Stacheln
   vor dem Ziel.
2. **Schluchten.** Zwei Kängurus zwischen breiter werdenden Schluchten, eine
   Landeplatte, die wegbricht (sofort weiterspringen), ein Trampolin über einen
   Canyon, ein Block aus dem Himmel, ein Rabe und Stacheln vor dem Ziel.
3. **Am roten Felsen.** Ein Block aus dem Boden am Start, hinauf auf den Felsen,
   ein Känguru und Stacheln oben, ein Rabe vor dem Jet-Flug über den Canyon, und
   das Ziel läuft zweimal weg.

**USA**

1. **Wüste.** Ein Saguaro, Steppenläufer, eine Schlucht mit einem Saguaro hinter
   der Landung, TNT, Stacheln, ein springender Steppenläufer und ein Ziel, das
   zurückspringt.
2. **Canyon.** Ein Block aus dem Himmel am Start, Rampen über Canyons, ein Block,
   der dort aus dem Boden schiesst, wo man landet, ein Steppenläufer im Matsch,
   eine Landeplatte, die wegbricht, ein Block aus dem Himmel, und das Ziel springt
   zurück.
3. **Goldmine.** Ein Block aus dem Boden am Start, ein Stollen mit TNT und einem
   steigenden Block, in dem Springen den Kopf kostet, ein Looping mit Stacheln
   dahinter, zwei Steppenläufer, eine breite Schlucht und ein Ziel, das
   zurückspringt.

**Paris**

1. **Über den Dächern.** Ein Kamin aus dem Himmel am Start, Lücken zwischen
   Häusern, ein Kamin auf dem Dach, eine Taube, eine Dachluke, ein Fass, Stacheln
   (Antennen) und ein Ziel, das über die letzte Lücke hüpft.
2. **Montmartre.** Steigende Dächer, eine Dachluke, eine Taube und ein Fass auf
   dem hohen Dach, eine Taube zusammen mit einem Kamin aus dem Himmel, eine breite
   Lücke vor TNT, und das Ziel springt zurück.
3. **Eiffelturm.** Ein Kamin aus dem Himmel am Start, ein Trampolin über einen
   Boulevard, ein zweites, das einen zurückwirft, eine Taube vor dem Jet-Flug auf
   die hohen Dächer, und das Ziel läuft über eine Lücke davon und wieder zurück.

**Schweiz**

1. **Alpweg.** Ein Schneeball am Start, eine erste Lawine von vorne mit Galerie,
   eine Lawine von hinten auf der langen Abfahrt, Stacheln an ihrem Fuss und ein
   Ziel, das zurückspringt.
2. **Pass.** Ein Schneeball auf der Passhöhe, eine Lawine von hinten treibt das
   Bike auf eine Brücke, die wegbricht, ein Rabe am Hang, eine Lawine von vorne
   mit Galerie, ein Block aus dem Himmel am Galerieausgang und ein Ziel, das
   zurückspringt.
3. **Gipfel.** Ein Schneeball am Start, der steilste Aufstieg mit einer Lawine von
   vorne, ein Trampolin über eine Gletscherspalte, eine Lawine von hinten, die zu
   schnell zum Davonfahren ist (unterstellen!), Stacheln, und das Ziel springt
   hinter die Stacheln zurück.

## Anschluss an das bestehende Spiel

Die [Leveldefinitionen](../../../src/levels/mod.rs) sind eine feste Liste mit
stabilen IDs; der [Spielstand](../../architecture/decisions/0005-save-progress-in-a-versioned-text-file.md)
merkt sich Fortschritt und Grabsteine pro Level-ID. Jedes Level nennt sein Land;
das Speicherformat ändert sich nicht.

Die Simulation kennt das Land nur für Zielbildschirm, HUD und Spielerauswahl.
Hintergrund, Farben und landesspezifisches Aussehen bestehender Fallen sind
Darstellung. Lawine und Treibsand bauen auf den Bausteinen aus
[ADR 0007](../../architecture/decisions/0007-compose-contact-effects-and-scripted-hazards.md)
auf; [ADR 0008](../../architecture/decisions/0008-group-levels-into-countries-with-presentation-themes.md)
hält die Entscheidungen fest.

## Woran wir den Versuch beurteilen

- Testspieler erkennen auf den ersten Blick, in welchem Land sie sind, und
  nennen nach einem Fehlversuch an Lawine oder Treibsand Ursache und Gegenaktion.
- Testspieler lachen über die Lügen und versuchen es nochmal; niemand gibt an einer
  Stelle auf, weil keine Antwort erkennbar ist.
- Die Lawine trifft auf ihrer Bahn und nirgends sonst; Davonfahren und Unterstellen
  funktionieren jeweils. Treibsand lässt ein Bike mit Tempo durch, versenkt ein
  stehendes und gibt es beim Sprung frei.
- Strecke und Gefahren bleiben vor allen Hintergründen gut lesbar.
- Bestehende Spielstände laden unverändert; wer die drei Übungslevels geschafft
  hat, startet im ersten Level von Ägypten.
- Geskriptete headless Fahrten schaffen jedes Länder-Level ohne Crash und lösen
  dabei jede Falle aus (ausser übersprungenen Luken). Ob es Spass macht und die
  Steigerung stimmt, wird im laufenden Spiel beurteilt.

# Sicherheit

Länder, Levels und Hintergründe sind fest ins Programm kompiliert. Es gibt
keinen Netzwerkzugriff, keine externen Inhalte und keine neuen Benutzereingaben.
Der Spielstand behält sein Format; neue Levels bekommen eigene stabile IDs.
Unbekannte Level-IDs in einer Datei werden wie bisher verworfen. Neue Grafiken
sind Formen im Code, keine nachgeladenen Dateien.

# Rabbit Holes

- **Kulissen-Grafik.** Zwei Ebenen aus einfachen Formen pro Land, eine feste
  Himmelsfarbe, wiederholbares Rauschen statt Zufall. Kein Farbverlauf über den
  Tag, keine ziehenden Wolken, keine Bilddateien.
- **Parallax und Kamera.** Die Ebenen folgen der Kamera waagrecht verlangsamt und
  senkrecht nur leicht. Keine Kamerafahrt und kein Zoom pro Land.
- **Lawinenphysik.** Keine Schnee- oder Flüssigkeitssimulation und kein
  Schneehaufen, der liegen bleibt. Eine Form auf fester Bahn mit fester
  Geschwindigkeit, die der Oberfläche samt Galeriedächern folgt und am Ende
  verschwindet.
- **Treibsand.** Kein echtes Einsinken des Physikkörpers. Ein Sinkwert steigt
  und fällt; gezeichneter Sand zeigt ihn. Sinkgeschwindigkeit und Bremswirkung
  sind so abgestimmt, dass „schnell durch“ klar funktioniert und „anhalten“ klar
  scheitert.
- **Steile Berge.** Mit Anlauf schafft das Bike Steigungen bis etwa 40°; die
  Schweizer Levels bleiben darunter. Keine Sonderphysik für Berge.
- **Schwierigkeit als Zahl.** Keine Schwierigkeitswerte, Formeln oder
  automatische Anpassung an den Spieler. Die Steigerung entsteht beim Levelbau.
- **Länder-Framework.** Kein Plugin pro Land, keine Landesdateien, keine
  Themen-Engine. Ein Land ist ein Enum-Wert am Level plus seine Farben und
  Kulissenformen.

# No-Gos

- Keine weiteren Länder als Übungsgelände, Ägypten, Australien, USA, Paris und
  Schweiz.
- Keine Weltkarte, keine Länder- oder Levelauswahl, kein Freischalten ausser der
  festen Reihenfolge.
- Keine Änderungen an den bestehenden drei Levels ausser ihrem Hintergrund.
- Keine landesspezifische Steuerung oder Physik: kein Eis, kein Rutschen, keine
  anderen Bikes.
- Keine landesspezifischen Power-ups.
- Keine Menschen, Volksfiguren oder Klischees über die Bewohner eines Landes; ein
  Land zeigt sich über Landschaft, Tiere und Bauwerke.
- Keine echten Marken, Logos oder Grafiken aus anderen Spielen.
- Kein Sound, keine Musik, kein Schneefall, kein Sandsturm und keine anderen
  Partikel.
- Kein Schwierigkeitsregler und kein leichter oder schwerer Modus.
