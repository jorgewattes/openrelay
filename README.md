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
  - **Proteções** (vermelho): fusível, disjuntor de 1 polo, disjuntor-motor de 3 polos e relé térmico.
  - **Motores** (verde-escuro): motor monofásico/CC e motor trifásico.
- Ligação por fios com cantos automáticos, junções marcadas e bornes livres destacados.
- Identificação automática (KM1, S1, H1, Q1…) e numeração dos bornes (13-14, 21-22, A1-A2, 1-2…).
- Contatos **seguem** a bobina, o temporizador ou o disjuntor com a mesma identificação.
- Selecionar, mover (os fios acompanham), girar, excluir, desfazer e refazer.

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
- Português e inglês (menu Ver → Idioma).

## Compartilhar por link

Como no simulador Falstad, um circuito pode ser enviado como **link**: **Arquivo → 🔗 Compartilhar link** (ou ☰ MENU no celular).

- O circuito inteiro vai **dentro do próprio link**, compactado e codificado no endereço depois de `?c=`. Não é criado nenhum arquivo nem conta.
- Quem abre o link recebe uma **cópia** para simular e alterar; **o link original nunca muda**.
- Para abrir no programa de computador um link recebido: **Arquivo → 📥 Abrir circuito de um link**.

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

## Como usar

1. Escolha um componente na lista (ou em **Componentes** no celular) e clique/toque na folha para colocá-lo. **R** gira.
2. Use a ferramenta **Fio** (W): clique num borne e depois em outro.
3. Dê aos contatos a mesma identificação da bobina que os aciona (ex.: KM1).
4. Pressione **▶ Simular** (F5) e opere os botões e chaves.

Atalhos no computador: `Ctrl+Z` desfazer · `Ctrl+Y` refazer · `Del` excluir · `R` girar · `W` fio · `V` selecionar · `F5` simular · `Esc` cancelar · roda do mouse = zoom · botão do meio/direito = mover a folha.

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
