<!--
Sjabloon voor het Bouwboek van Headless Hug: de constructieve beoordeling
volgens NEN-EN 13814, in de hoofdstukindeling van ENS (Open Claw, versie 02,
17-5-2026). Gerald schrijft het concept, ENS toetst en tekent.

Elk veld tussen dubbele accolades vult de export uit dezelfde bouw als de JSON, de DXF's en het
lasdocument (src/bouwboek.rs):

    cargo test --release --lib export_headless_hug_package -- --ignored

Elk hoofdstuk zegt onder de kop waar het vandaan komt:
  gegenereerd  getallen en tabellen uit de bouw, niet met de hand wijzigen
  Gerald       tekst van Gerald, waar nodig overgenomen uit Open Claw
  ENS          door de ingenieur in te vullen of te toetsen
"TE DOEN" markeert wat nog open staat.

Nieuwe termen, nog niet door Peter gebruikt, na te vragen: ring met nok en
dwarsbuis, knoopplaat, gaffelstang, ophanging knoopplaat, scharnier,
tuidraad, grondanker.
-->

# Bouwboek Headless Hug

| | |
|---|---|
| Project | TE DOEN: projectnummer ENS |
| Opdrachtgever | Beautiful Code B.V. |
| Versie - datum | 00 – {{datum}} (concept, gegenereerd) |
| Geldig tot | TE DOEN |
| Ingenieur | TE DOEN |
| Controle | TE DOEN |

## 1 Samenvatting

*Bron: Gerald, eindoordeel ENS*

Voor de tensegrity Headless Hug is een constructieve beoordeling uitgevoerd van de tensegrity constructie,
de voeten, de tuidraden en de bijbehorende verbindingen. De constructie bestaat uit aluminium drukstaven en
stalen trekkabels en is beoordeeld met een geometrisch niet-lineair rekenmodel in RFEM 6, aangevuld met
detailtoetsen van maatgevende onderdelen.

TE DOEN (ENS): het oordeel, na de berekening.

Algemene informatie van de constructie:

| Omschrijving | |
|---|---|
| Afmetingen | Lengte: {{lengte}} m<br>Breedte: {{breedte}} m<br>Hoogte: {{hoogte}} m |
| Constructie | Tensegrity constructie van {{aantal_staven}} aluminium staven en {{aantal_kabels}} stalen kabels, staand op {{aantal_voeten}} voeten en gehouden door {{aantal_tuidraden}} tuidraden naar grondankers. |
| Windbelasting | TE DOEN (ENS): stuwdruk en hoogte. |
| Ballast | Geen. TE DOEN: grondankers van de tuidraden en de bevestiging van de voeten. |
| Extra opmerking | TE DOEN: voorwaarden voor gebruik (ondergrond, ankers, klimmen, geen decoratie of rigging loads). |

## 2 Uitgangspunten

*Bron: Gerald, overgenomen uit Open Claw*

Voor de beoordeling in dit rapport zijn de volgende uitgangspunten gehanteerd:

- De geometrie van de constructie is aangehouden volgens de in Hfdst. 5 opgenomen tekeningen en de
  bijgevoegde modelgegevens (JSON).
- De toegepaste materialen, onderdelen en doorsneden zijn aangehouden volgens Hfdst. 6 van dit rapport.
- De constructie is beoordeeld voor de in dit rapport beschreven configuratie, oplegging en verankering.
- De constructie mag uitsluitend worden gebruikt bij windsnelheden tot de in Hfdst. 7 vermelde limieten.
- Bij overschrijding van deze limieten dient de omgeving van de constructie in een straal van TE DOEN meter
  vanuit het midden te worden geëvacueerd.

## 3 Informatie eigenaar

*Bron: Gerald*

| Omschrijving | |
|---|---|
| Naam | Beautiful Code B.V. |
| Adres | TE DOEN |
| Plaats | TE DOEN |
| Contactpersoon | Gerald de Jong |
| Email | TE DOEN |
| Telefoon | TE DOEN |

## 4 Informatie producent

*Bron: Gerald*

| Omschrijving | |
|---|---|
| Naam | Beautiful Code B.V. |
| Contactpersoon | Gerald de Jong |
| | TE DOEN: als bij de eigenaar, of de werkplaats |

## 5 Geometrie van de constructie

*Bron: Gerald, met getallen uit de bouw*

De constructie betreft een tensegrity constructie, opgebouwd uit aluminium drukstaven en stalen trekkabels
die gezamenlijk in evenwicht zijn. De staven hebben een rustlengte tussen {{staaflengtes}}. De kabels hebben
een rustlengte tussen {{kabellengtes}}. Kabels die te kort zijn om te maken zijn uitgevoerd als gaffelstang
van M8 draadstang ({{aantal_gaffelstangen}} stuks, waarvan sommige een knoopplaat ophangen).

De constructie staat op {{aantal_voeten}} voeten: een stangkop op het einde van een staaf, met een pen naar
de grond. Zij wordt overeind gehouden door {{aantal_tuidraden}} tuidraden naar grondankers. TE DOEN: de
bevestiging van de voeten en de ankers zijn nog niet ontworpen.

De staven zijn opgebouwd uit drie in elkaar schuivende buizen, als bij Open Claw: een buitenbuis 40x3 mm met
aan elk einde een binnenbuis 33x3 mm, verstelbaar met pennen in de gaten. Op elk staafeinde zit een kop met
een M12 bout. Daarop draait een stapel ringen, elk met een nok en een dwarsbuis, waaraan de gaffelterminal
van een kabel scharniert; afstandhouders van POM scheiden de ringen. Beide hoeken zijn vrij, dus elke ring is
gelijk. Waar meerdere kabels samenkomen nemen {{aantal_knoopplaten}} knoopplaten twee kabels van een volle
stapel over. Op {{aantal_scharnieren}} plaatsen zijn twee staven met een scharnier verbonden.

![Figuur 1 3D-aanzicht](images/3d.png)

*Figuur 1 3D-aanzicht, met de tuidraden naar de grondankers*

![Figuur 2 Vooraanzicht](images/voor.png)

*Figuur 2 Vooraanzicht*

![Figuur 3 Zijaanzicht](images/zij.png)

*Figuur 3 Zijaanzicht*

![Figuur 4 Bovenaanzicht](images/boven.png)

*Figuur 4 Bovenaanzicht*

TE DOEN: foto's van een proefstuk: een kop met ringen, een knoopplaat, een scharnier, een voet.

## 6 Doorsneden en materialen

*Bron: gegenereerd; materiaalkwaliteiten ENS*

Onderstaande tabel bevat de constructiedelen, met hun aantal in deze bouw.

{{tabel_onderdelen}}

De onderdelen van de kopverbindingen, de staven en de verbindingen, elk met de letter waaronder het in de
applicatie staat (toets P):

{{figuren_onderdelen}}

TE DOEN (ENS): staalkwaliteit van ringen, dwarsbuizen, knoopplaten en gaffels; bout- en penkwaliteit.
Materiaaleigenschappen worden beschreven in Hfdst. 11 van dit rapport.

## 7 Toelaatbare windsnelheden

*Bron: ENS*

TE DOEN (ENS): basisstuwdruk en de afleiding van de windsnelheden in Bijlage 15.1.

### 7.1 Limiet windsnelheden

Voor windsnelheden hoger dan de in onderstaande tabel vermelde waarden zijn de sterkte en/of stabiliteit van
de constructie niet meer gewaarborgd. Bij overschrijding van deze limieten dient de omgeving van de
constructie binnen een straal van TE DOEN m vanaf het middelpunt te worden geëvacueerd.

| Omschrijving | Bebouwd NL: cat. III | Onbebouwd NL: cat. II | Zee of kust NL: cat. 0 |
|---|---|---|---|
| A Beaufort (indicatief) | TE DOEN | TE DOEN | TE DOEN |
| B 10 min. gemiddelde windsnelheid | TE DOEN | TE DOEN | TE DOEN |
| C Piekwindsnelheid (windvlaag) | TE DOEN | | |

### 7.2 Beschrijving

Als bij Open Claw: terreincategorieën en de waarden A, B en C.

## 8 Toegepaste normen en voorschriften

*Bron: overgenomen uit Open Claw*

De constructie valt onder de categorie attractietoestellen en is beoordeeld volgens NEN-EN 13814. Voor de
bepaling van belastingen, stabiliteit en materiaalweerstanden zijn daarnaast, voor zover van toepassing,
onderstaande normen en nationale bijlagen aangehouden.

- NEN-EN 1990-serie – Grondslagen van het constructief ontwerp
- NEN-EN 1991-serie – Belastingen op constructies
- NEN-EN 13814 – Veiligheid van attractietoestellen
- NEN-EN 1993-serie – Ontwerp en berekening van staalconstructies
- NEN-EN 1999-serie – Ontwerp en berekening van aluminiumconstructies

## 9 Belastingen

### 9.1 Eigen gewicht

*Bron: gegenereerd*

Het gewicht van de constructie, uit de massa's van de onderdelen. In het rekenmodel wordt het per knoop
aangebracht vanuit de JSON, met het eigen gewicht van RFEM uit.

{{tabel_gewicht}}

### 9.2 Voorspanning

*Bron: gegenereerd, toelichting Gerald, toetsing ENS*

De voorspanning wordt aangebracht door de staven langer te maken dan de lengte waarbij de kabels net strak
staan. Elke staaf wordt op de werf op zijn rustlengte gezet met de pennen in de aangegeven gaten; er is geen
gereedschap en geen meting van kracht nodig. Staand onder eigen gewicht dragen de staven tot {{max_staafkracht}}
kN druk en de kabels tot {{max_kabelkracht}} kN trek.

In het rekenmodel komt de voorspanning binnen als opgelegde rek van elk element, rustlengte / lengte − 1
(`build_rfem.py`, `PRESTRESS`).

{{tabel_staven}}

De pennen staan per einde zoals de bouwploeg ze telt, vanaf 1: het gat in de binnenbuis voor de buitenste pen,
dan de twee gaten in de buitenbuis, bijvoorbeeld `12:3-6`. De afwijking is hoeveel de staaf met die pennen
van zijn rustlengte afwijkt.

{{tabel_trekelementen}}

TE DOEN (ENS): of dit niveau van voorspanning voldoet, met marge voor verlies door initiële rek en
temperatuur.

### 9.3 Sneeuw belasting

Volgens NEN-EN 13814, par. 4.3.3.5, wordt geen rekening gehouden met sneeuwbelasting. De vorm van de
constructie geeft geen aanleiding tot relevante sneeuwophoping.

### 9.4 Gebruiksbelasting

TE DOEN: zonder trusskolommen staan de onderste staven binnen bereik. Antiklimmaatregelen, of een
klimbelasting.

### 9.5 Wind belasting

*Bron: ENS*

TE DOEN (ENS): windrichtingen; de figuur is spiegelsymmetrisch, niet drievoudig, dus meer richtingen dan bij
Open Claw.

#### 9.5.1 Windvormfactoren buizen

Als bij Open Claw, voor buis 40 mm.

#### 9.5.2 Windbelasting op de tuidraden

TE DOEN (ENS).

### 9.6 Beweging

Headless Hug beweegt: de tuidraden worden aangedreven, in het model met een periode van ongeveer 4 s
(`docs/parts.md`, bevinding 3). Daarbij gaan enkele kabels per cyclus slap. TE DOEN: de aandrijving
ontwerpen; TE DOEN (ENS): een eigen belastinggeval, en vermoeiing.

## 10 Belastingcombinaties

*Bron: ENS*

Als bij Open Claw, volgens NEN-EN 13814 art. 4.3.6 en 4.5.1, met de belastinggevallen van Hfdst. 9.

## 11 Materiaaleigenschappen

*Bron: ENS, uit Open Claw*

Aluminium EN AW-6060 T66, staalkabel 6 mm 7x19 verzinkt, S235 en de kwaliteiten van Hfdst. 6.

## 12 Toetsing

*Bron: ENS, met krachten uit RFEM*

### 12.1 Rekenprogramma en instellingen

Het rekenmodel wordt opgebouwd uit de JSON met `build_rfem.py`, niet meer via Rhino en Grasshopper.
TE DOEN (ENS): instellingen, imperfecties.

### 12.2 Toets aluminium buizen

- 12.2.1 Druk, buiging en initiële uitbuiging
- 12.2.2 Druk in netto doorsnede ter plaatse van de pen
- 12.2.3 Stuik in buis t.p.v. pin
- 12.2.4 Afschuiving en buiging van de pen

### 12.3 Ring met nok en dwarsbuis

TE DOEN (ENS): wand van de dwarsbuis en de las aan de nok. Vervangt de toets van de lip bij Open Claw.

### 12.4 Kracht in kabel

### 12.5 Gaffelstangen en knoopplaten

TE DOEN (ENS): M8 draadstang, gaffels en knoopplaten.

### 12.6 Scharnieren

TE DOEN (ENS): GN 751 / GN 752 M12, volgens het productblad.

## 13 Toets stabiliteit

*Bron: ENS*

Kantelen, schuiven en liften, nu om de voeten en de tuidraden in plaats van om trusskolommen met
betonblokken.

{{tabel_voeten}}

{{tabel_tuidraden}}

## 14 Toetsingen verbinding en verankering

*Bron: ENS*

- 14.1 Stangkop INA GIKR12-PW aan de voeten
- 14.2 Bevestiging van de voeten aan de grond (TE DOEN: ontwerp)
- 14.3 Grondankers van de tuidraden (TE DOEN: ontwerp)

## 15 Bijlagen

- 15.1 Windsnelheden
- 15.2 Beaufortschaal
- 15.3 Productbladen: kabel, gaffelterminal, stangkop, scharnier
- 15.4 RFEM-berekening
- 15.5 Eurocode checks
- 15.6 Modelgegevens: de JSON, `build_rfem.py`
