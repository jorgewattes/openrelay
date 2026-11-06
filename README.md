# SimuEletro ⚡

**Simulador de comandos elétricos que funciona no navegador, no computador e no celular.**

O SimuEletro é uma releitura moderna dos simuladores clássicos de comandos elétricos (no estilo do CADe_SIMU). Com ele você desenha diagramas de força e de comando com símbolos no padrão IEC e depois **simula o circuito**: aperta botões, liga contatores, vê os motores girarem e as lâmpadas acenderem, com os fios mudando de cor conforme a tensão.

👉 **Use online:** https://jorgewattes.github.io/simueletro/

| Computador | Celular |
|---|---|
| ![Tela no computador](docs/tela-computador.png) | ![Tela no celular](docs/tela-celular.png) |

---

## Para que serve

- Estudar e ensinar **comandos elétricos industriais**: partida direta, selo, intertravamento, reversão e temporização.
- Testar um diagrama **antes de montar o painel**: o simulador mostra o que liga, o que desliga e onde há curto-circuito.
- Praticar em qualquer lugar, inclusive **no celular**, sem instalar nada.

## Funcionalidades

### Desenho
- Folha com grade, moldura, colunas (A–H) e linhas (1–5), como numa folha de projeto.
- **Componentes organizados em categorias** (cada uma com sua cor na biblioteca):
  - **Fontes** (verde): fases L1, L2, L3, neutro N, terra PE, +24 V e 0 V CC.
  - **Bobinas** (amarelo): bobina de contator, temporizador ao trabalho e ao repouso.
  - **Auxiliares** (azul): contatos auxiliares NA e NF.
  - **Comandos** (roxo): botão NA (liga), botão NF (desliga), chave seletora, emergência e fins de curso NA/NF.
  - **Cargas** (marrom): lâmpada sinalizadora (5 cores) e sirene.
  - **Contatores** (ciano): contatos principais de 3 polos.
  - **Proteções** (vermelho): fusível, disjuntor de 1 e 2 polos, disjuntor-motor de 3 polos, **DR bipolar e tetrapolar** (desarma com fuga de corrente para a terra ou com um clique, como o botão de teste) e relé térmico.
  - **Motores** (verde-escuro): motor monofásico/CC e motor trifásico.
- Ligação por fios com cantos automáticos, junções marcadas e bornes livres destacados.
- Identificação automática (KM1, S1, H1, Q1…) e numeração dos bornes (13-14, 21-22, A1-A2, 1-2…).
- Contatos **seguem** a bobina, o temporizador ou o disjuntor com a mesma identificação.
- **Botoeiras com vários contatos:** dê a mesma identificação (ex.: S1) a vários contatos NA/NF e escolha o **Contato nº** nas propriedades (13-14, 21-22, 33-34…); na simulação todos mudam juntos.
- Selecionar, mover (os fios acompanham), girar, excluir, desfazer e refazer.
- **Copiar, recortar e colar** (Ctrl+C, Ctrl+X, Ctrl+V) e duplicar (Ctrl+D): o trecho colado vai para onde está o mouse; bobinas e botões colados ganham nova identificação (KM1 → KM2) e os contatos colados junto passam a seguir a nova bobina. Também funciona entre duas janelas/abas do SimuEletro.

### Simulação
- Fios com tensão coloridos: **vermelho** = fase / +24 V, **azul** = neutro / 0 V, **verde tracejado** = terra.
- Botões funcionam enquanto pressionados; chaves, fins de curso, emergência, disjuntores e relés térmicos mudam de estado com um clique ou toque.
- Contatores com selo e intertravamento, temporizadores com contagem visível.
- Motor trifásico gira no sentido da sequência de fases (inverter duas fases inverte o sentido) e avisa quando falta uma fase.
- **Detecção de curto-circuito**, com aviso e destaque no trecho com problema.
- Painel de estado com contatores, temporizadores, motores e lâmpadas.

### Telas
- **Computador:** menus, barra de ferramentas, lista de componentes à esquerda e propriedades à direita.
- **Celular / tela estreita** (tema escuro, no estilo do app SimuRelay): barra superior com **MENU · PEÇAS · SIMULAR · FIO · MAIS · DESFAZER · REFAZER**; a biblioteca **PEÇAS** abre por cima da folha com um seletor de **categorias coloridas** e uma grade de símbolos. Com uma peça selecionada, a barra muda para **VALORES · GIRAR · COPIAR · EXCLUIR**. Há botões de zoom **− / +** no canto; arraste com um dedo para mover a folha, use a pinça para o zoom e **arraste de um borne a outro para ligar um fio**.
- **Modo claro e modo escuro** (Ver → Tema, ou MAIS no celular). No modo Automático, o celular segue o padrão do aparelho e o computador fica claro.
- Português e inglês (menu Ver → Idioma).

### Exportar e imprimir
- **Arquivo → Exportar imagem (tudo)** ou **(seleção)**: salva um PNG do desenho, em fundo branco.
- **Arquivo → Imprimir / salvar em PDF** (Ctrl+P): cria uma página A4 com o título e o diagrama, pronta para imprimir.
- No navegador e no celular o arquivo é baixado; no computador abre a janela para escolher onde salvar.

## Tensões

As tensões são escolhidas numa **lista suspensa** nas propriedades: só existem valores tabelados, sem digitar números e sem cálculos complexos.

| Peça | Opções |
|---|---|
| Fases L1/L2/L3 (sistema CA) | **220 V / 380 V** ou **127 V / 220 V** (fase / linha) |
| Fonte CC (+V) | **+24 V** ou **+12 V** |
| Lâmpadas/LEDs, sirene, bobinas e temporizadores | 220, 127, 380 ou 24 V CA · 24 ou 12 V CC |
| Motor monofásico/CC (fechamento interno) | 220 ou 127 V CA · 24 ou 12 V CC |
| Motor trifásico (fechamento interno) | 380, 220 ou 440 V entre fases |
| Motor trifásico 6 terminais | Δ 220 / Y 380 V · Δ 380 / Y 660 V · Δ 127 / Y 220 V |

Na simulação, a tensão aplicada é comparada com a nominal (tolerância de ±10%):

- **Em sobretensão nada funciona:** sinaleiras/LEDs **queimam** (ficam queimadas até parar a simulação), bobinas **não ligam** e motores **não giram**.
- **Em subtensão:** sinaleiras/LEDs acendem **mais claras (fracas)**, motores **giram mais devagar** e bobinas de contator **não ligam**.
- CA no lugar de CC (ou o contrário): a peça não funciona.
- **Motor de 6 terminais:** detecta se foi fechado em **estrela** (U2-V2-W2 juntos) ou **triângulo** (U1-W2, V1-U2, W1-V2).
- Os avisos aparecem **ao lado da peça** na folha e no painel de estado.

O exemplo **Arquivo → Exemplos → Tensões: motor Y/Δ e lâmpadas** mostra tudo isso.

## Compartilhar por link

Como no simulador Falstad, um circuito pode ser enviado como **link**: **Arquivo → 🔗 Compartilhar link** (ou ☰ MENU no celular).

- O circuito inteiro vai **dentro do próprio link**, compactado e codificado no endereço depois de `?c=`. Não é criado nenhum arquivo nem conta.
- Quem abre o link recebe uma **cópia** para simular e alterar; **o link original nunca muda**.
- Para abrir um circuito recebido, basta acessar o link.

## Se o app não abrir

O SimuEletro precisa de um navegador com **WebAssembly e WebGL** (Chrome, Edge, Firefox ou Safari atualizados). Se a página mostrar um aviso:

- abra o link no navegador do celular, e não dentro de outro app (WhatsApp, Instagram etc.);
- atualize a página;
- confira se a "aceleração de hardware" do navegador está ligada.

A mensagem na tela mostra o motivo exato do erro, o que ajuda a corrigir.

## Exemplos prontos

No menu **Arquivo → Exemplos** (ou ☰ no celular):

1. **Partida direta com selo:** motor trifásico com disjuntor-motor, contator, botões liga/desliga e lâmpadas de "em funcionamento" e "disjuntor desarmado".
2. **Reversão de motor:** dois contatores com intertravamento elétrico, que invertem as fases L1 e L3.
3. **Lâmpada temporizada:** temporizador ao trabalho que acende uma lâmpada alguns segundos depois da partida.
4. **Tensões:** motor Y/Δ e lâmpadas com tensões nominais diferentes.
5. **Partida estrela-triângulo temporizada:** contatores KM1 (linha), KM2 (estrela) e KM3 (triângulo), temporizador KT1 e intertravamentos. O motor (Δ380/Y660 V) parte em estrela com tensão reduzida e, após 5 s, passa para triângulo.

## Como usar

1. Escolha um componente na lista (ou em **Componentes** no celular) e clique/toque na folha para colocá-lo. **R** gira.
2. Use a ferramenta **Fio** (W): clique num borne e depois em outro.
3. Dê aos contatos a mesma identificação da bobina que os aciona (ex.: KM1).
4. Pressione **▶ Simular** (F5) e opere os botões e chaves.

Atalhos no computador: `Ctrl+C` copiar · `Ctrl+X` recortar · `Ctrl+V` colar · `Ctrl+D` duplicar · `Ctrl+P` imprimir/PDF · `Ctrl+Z` desfazer · `Ctrl+Y` refazer · `Del` excluir · `R` girar · `W` fio · `V` selecionar · `F5` simular · `Esc` cancelar · roda do mouse = zoom · botão do meio/direito = mover a folha.

## Como executar localmente

É preciso ter o [Rust](https://rustup.rs) instalado.

```bash
# Versão para computador (Windows, Linux, macOS)
cargo run --release

# Abrir com o layout de celular
cargo run --release -- --mobile

# Versão web (igual à do GitHub Pages)
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve
```

No computador, os circuitos podem ser salvos e abertos como arquivos `.json` (Arquivo → Salvar / Abrir).

## Publicação

Cada envio para a branch `main` dispara o fluxo [`.github/workflows/pages.yml`](.github/workflows/pages.yml), que compila a versão web com o [Trunk](https://trunkrs.dev) e publica no GitHub Pages.

## Tecnologia

- [Rust](https://www.rust-lang.org/) com [egui / eframe](https://github.com/emilk/egui): a mesma base de código roda como programa de computador e como WebAssembly no navegador.
- A simulação junta os pontos ligados por fios e contatos fechados em "redes", identifica a tensão de cada rede (fase, neutro, CC) e decide o que fica energizado, repetindo até os contatores se estabilizarem.

```
src/
├── main.rs      # início no computador e no navegador
├── app.rs       # telas (computador e celular), ferramentas e desenho da folha
├── circuit.rs   # componentes, fios e biblioteca de peças
├── draw.rs      # símbolos elétricos (IEC)
├── sim.rs       # motor de simulação (+ testes automáticos)
└── examples.rs  # circuitos de exemplo
```

Testes da simulação: `cargo test`.

## Aviso

Projeto educacional independente, inspirado nos simuladores de comandos elétricos. Não tem ligação com os autores do CADe_SIMU nem do SimuRelay. Não substitui projeto elétrico nem as normas (NBR 5410, NR-10 etc.).
