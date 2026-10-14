# CircuitCraft — avvio rapido
1. Estrai **tutto** lo ZIP.
2. Apri **Start-CircuitCraft.bat**, oppure **windows/circuitcraft.exe**.
3. Il filtro RC è già disegnato: premi **F5** per vederne la risposta in frequenza.
4. In **Examples** trovi anche partitore e raddrizzatore.
5. In **PCB layout** trovi le schede dei tre esempi già collegate.

## Disegnare
- **File → New empty schematic**.
- Scegli un componente a sinistra e posalo sulla griglia.
- **R** ruota; seleziona il componente per cambiare valore, modello, parametri e footprint.
- **W** collega i terminali; **Esc** termina il filo.
- **G** aggiunge ground. **J** collega un incrocio. **L** assegna un nome alla rete.
- Scegli l'analisi e premi **Run**. **Linear DC** usa il solver Rust per circuiti lineari.
- Salva con **Ctrl+S**. **Ctrl+Z / Ctrl+Y** annullano e ripristinano.
- Ruotare o spostare un componente già collegato richiede di aggiornare i fili.

## Scheda PCB
- **Synchronize from schematic** aggiorna footprint e reti.
- **Route** disegna piste. Partendo da un pad, seleziona automaticamente la sua rete.
- Usa **Top / Bottom** e **Via** per i due strati.
- **Run design rule check** mostra corti, problemi di distanza e collegamenti mancanti.
- **File → Export Gerber + drills** crea i file dentro la cartella di destinazione.
- I footprint sono generici: controlla dimensioni e numeri dei pin sul datasheet reale prima di produrre una scheda.

## Modelli e dati
- **SPICE** permette di aggiungere modelli, parametri e subcircuiti.
- Importare una netlist non ricostruisce lo schema grafico. Il selettore indica quale sorgente viene simulata.
- Usa **Include base directory** per i file modello con percorsi relativi.
- **BOM** mostra la distinta; il menu File esporta CSV, SPICE, SVG e Gerber.
- Le impostazioni di analisi appartengono alla sessione; geometria, modelli, netlist, percorso degli include e note sono salvati.

Il pacchetto Windows include ngspice 47 e i sorgenti Rust: puoi lavorare offline.
È una prima versione con una libreria limitata. Restano da sviluppare autorouting, copper pour, gerarchia multischema, PCB 3D, importazione nativa KiCad/Altium e simulazione di microcontrollori.

Per verificare automaticamente il motore sul tuo Windows:
```powershell
.\scripts\Test-Windows-Engine.ps1
```

## Novità 0.2
- **Waveforms → Component parameter sweep**: abilita lo sweep, scegli R1, scrivi `500,1k,2k` e premi Run.
- **Noise spectrum**: scegli VOUT e V1 per il rumore.
- **Signal math / gain**: calcola differenza, rapporto o prodotto tra segnali.
- **Custom footprint / pad editor**: seleziona un componente nello schema e modifica i pad; poi sincronizza il PCB.
- Il menu File esporta misure, sweep, coordinate di piazzamento e SVG PCB.
- Le istruzioni dettagliate e le definizioni delle misure sono in `docs/ADVANCED_FEATURES.md`.
