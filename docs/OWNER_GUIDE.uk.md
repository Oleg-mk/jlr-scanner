# Інструкція власника: ключ, копії бібліотеки, тестувальники, збірки

Стан на 2026-09-06 (ADR-0019). Усе, що тут описано, виконується на
власному ПК власника; Docker Desktop має бути запущений, бо інструменти
Rust на цьому ПК працюють лише в контейнері.

## 1. Ключ видачі

- Файл: `C:\Users\<you>\.jlr-scanner\library-issuer.key`, один рядок із
  64 символів. Резервна копія на флешці зроблена 2026-09-06.
- Ніколи: в репозиторій, у чат, у пошту, у спільну хмару. У застосунку
  лежить лише публічна половина ключа (id `382146ce`), нею він перевіряє
  підпис на штампі; без приватної половини дійсної копії не зробити.
- Якщо ключ загублено: зробити новий командою нижче, публічну половину
  додати до списку `TRUSTED_ISSUER_KEYS` у
  `crates/diagnostic-session/src/issue.rs`, зібрати застосунок і видати
  всі копії заново. Старі копії доживуть до своєї дати.

```bash
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "C:/Users/<you>/.jlr-scanner:/issuer" rust:1.98-slim sh -c 'cargo run --release -q -p diagnostic-session --example stamp_library -- --new-key /issuer/library-issuer-2.key'
```

## 2. Видати копію тестувальнику

Одна команда з кореня репозиторію:

```bash
powershell -File scripts/stamp-library.ps1 -IssuedTo "Ім'я Прізвище"
```

Що вона робить: бере бібліотеку з `Downloads\jlr-scanner-library`,
пише копію в `Downloads\jlr-scanner-issued\<ім'я>`, ставить штамп на 30
днів із підписом, читає копію так, як прочитає застосунок, і лише тоді
пакує в `Downloads\jlr-scanner-issued\jlr-scanner-library-<ім'я>.zip`.
Якщо перевірка не пройшла, команда зупиняється і zip не створюється.

- Інший термін: `-Days 60`. Собі: `-IssuedTo "Oleg" -Days 365` (уже
  видано до 2027-09-06, код `5050-F8EF`, папка `jlr-scanner-issued\Oleg`).
- Тестувальнику надіслати три речі, одним каналом на одну людину:
  1. zip його копії;
  2. інсталятор потрібної системи з останньої збірки
     (`Downloads\jlr-scanner-build-<sha>\`), хеш є в `CURRENT_STATE.md`;
  3. `docs/TESTER_GUIDE.uk.md` або `docs/TESTER_GUIDE.md`.
- Записати: кому, коли, код копії, до якої дати. Код видно у файлі
  `issued_to.json` копії і в кожному звіті, який тестувальник надішле.

## 3. Продовжити або перевірити копію

- Продовжити означає видати знову тією самою командою: буде новий код і
  нова дата. Стара копія працює до своєї дати, далі застосунок її
  відхиляє і просить нову; за тиждень до дати він попереджає сам.
- Перевірити будь-яку папку так, як це зробить застосунок:

```bash
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "C:/Users/<you>/Downloads:/downloads" rust:1.98-slim sh -c 'cargo run --release -q -p diagnostic-session --example stamp_library -- --check "/downloads/jlr-scanner-issued/Ім_я"'
```

Відповідь `state: Loaded` і `issue: Matches` означає, що копія дійсна.

## 4. Що робити зі звітами тестувальників

Тестувальник надсилає файл звіту сесії, файл захоплення і кілька рядків
про авто. Звіт перетворюється на захоплений маніфест бібліотеки:

```bash
docker run --rm -v "C:/Users/<you>/Desktop/jlr-scanner:/w" -w /w -v jlr-cargo-registry:/usr/local/cargo/registry -v "C:/Users/<you>/Downloads:/downloads" rust:1.98-slim sh -c 'cargo run --release -q -p report-intake --example intake -- /downloads/reports/report.json /downloads/jlr-scanner-library'
```

Результат `captured-<id>.json` лягає в папку бібліотеки. Наступні копії
вже несуть його, і в огляді автомобіля підтверджені модулі показуються як
`CAPTURE_VALIDATED`. Це і є накопичення нашої власної, виміряної бази.

## 5. Збірки

- Кожен пуш у гілку запускає CI; збірка macOS тарифікується вдесятеро.
  Пуші, що торкаються лише документів, збірку не запускають.
- Якщо GitHub пише, що джоби не стартували через оплату: Settings →
  Billing and plans, підняти spending limit або дочекатися нового місяця.
  Після цього збірку перезапускає `gh run rerun <id>` без нового коміту.
- Готові інсталятори: `Downloads\jlr-scanner-build-<sha>\`, хеші записані
  в `CURRENT_STATE.md`.
- Перша збірка після `b3fa55d` приймає лише штамповані копії: стару
  папку `jlr-scanner-library` вона відхилить, бери копію з
  `jlr-scanner-issued\Oleg`.

## 6. Якщо щось не так

- «Термін копії минув» у тестувальника: видати знову, розділ 2.
- «У цій папці немає штампа видачі»: це папка-джерело без штампа, треба
  дати копію.
- Копію змінили або переслали з іншого ПК зі зміненими файлами: застосунок
  каже «дані не збігаються зі штампом», видати знову.
- Загублений ключ: розділ 1.
