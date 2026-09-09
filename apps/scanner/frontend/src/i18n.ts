import { createContext, useContext } from "react";

/**
 * Interface language. The application's own words come in English, Russian
 * and Ukrainian. The vehicle data's words — SDD's fault-code wording,
 * parameter names, module names — are shown as the loaded library has them:
 * SDD's text database carries twelve languages, Russian among them and
 * Ukrainian not, so module names and failure-type wording follow the
 * interface into Russian and stay English for Ukrainian (see
 * `dataLanguage`); fault-code descriptions exist in English only.
 *
 * Strings are keyed by their English text, so a missing translation shows
 * the English rather than a key, and tests read the English they assert.
 * `{name}` placeholders are filled from the params.
 */
export type Language = "en" | "ru" | "uk";

export const LANGUAGE_STORAGE_KEY = "prowlone.language";

export const languages: Array<{ id: Language; label: string }> = [
  { id: "en", label: "English" },
  { id: "ru", label: "Русский" },
  { id: "uk", label: "Українська" },
];

/**
 * SDD's language code whose data text matches the interface language, or
 * null when SDD has none for it (Ukrainian) and English is shown instead.
 */
export function dataLanguage(language: Language): "eng" | "rus" | null {
  switch (language) {
    case "ru":
      return "rus";
    case "en":
      return "eng";
    default:
      return null;
  }
}

/**
 * Our own wording's language code for the interface language, or null for
 * English, which is the loaded data's own. Unlike `dataLanguage` this knows
 * Ukrainian, because the text is ours and not SDD's.
 */
export function productLanguage(language: Language): "ukr" | "rus" | null {
  switch (language) {
    case "uk":
      return "ukr";
    case "ru":
      return "rus";
    default:
      return null;
  }
}

/**
 * A fault code's wording for the interface language: ours when we have it,
 * otherwise the loaded data's English. The English is returned as well, so
 * the caller can show it beside the translation — a tester quotes the
 * English, and every report carries only that.
 */
export function codeText(
  texts: Record<string, string> | undefined,
  english: string | null,
  language: Language,
): { shown: string | null; original: string | null } {
  const code = productLanguage(language);
  const ours = code !== null ? texts?.[code] : undefined;
  if (ours === undefined) return { shown: english, original: null };
  return { shown: ours, original: english };
}

/** Pick the data text for the interface language, English when SDD has none. */
export function dataText(
  texts: Record<string, string> | undefined,
  fallback: string | null,
  language: Language,
): string | null {
  const code = dataLanguage(language);
  if (texts !== undefined && code !== null && texts[code] !== undefined) return texts[code];
  return texts?.eng ?? fallback;
}

const uk: Record<string, string> = {
  // Header and session
  "Multi-platform vehicle diagnostics": "Кросплатформна діагностика автомобілів",
  "Simulator UI preview": "Попередній перегляд інтерфейсу (симулятор)",
  "Development preview only — no Mongoose or vehicle communication.":
    "Лише попередній перегляд для розробки — без Mongoose і без звʼязку з автомобілем.",
  "Adapter ready": "Адаптер готовий",
  "Adapter connected, board unverified": "Адаптер підключено, плату не перевірено",
  "Adapter found, not connected": "Адаптер знайдено, не підключено",
  "Adapter error": "Помилка адаптера",
  "No adapter": "Адаптера немає",
  "Library: {records} records": "Бібліотека: {records} записів",
  "Library failed to load": "Бібліотека не завантажилась",
  "Library: built-in only": "Бібліотека: лише вбудовані дані",
  "Saving…": "Збереження…",
  "Save session report": "Зберегти звіт сесії",
  "New session": "Нова сесія",
  "Start a new session?": "Розпочати нову сесію?",
  "The report, the survey and the reads of this session are dropped unless saved. The adapter stays connected and the library stays loaded.":
    "Звіт, огляд і читання цієї сесії буде втрачено, якщо їх не збережено. Адаптер лишається під'єднаним, бібліотека завантаженою.",
  "Start": "Розпочати",
  "Cancel": "Скасувати",
  Language: "Мова",
  done: "виконано",
  next: "наступний",
  "to do": "попереду",
  "One session, one report": "Одна сесія — один звіт",
  "Preparation": "Підготовка",
  "Module network": "Мережа модулів",
  "Listening and standard OBD-II": "Прослуховування і стандартне OBD-II",
  "Session report": "Звіт сесії",
  "Step {list}": "Крок {list}",
  "Steps {list}": "Кроки {list}",
  "Go to step {index}": "Перейти до кроку {index}",
  "One file with the survey, every capture and every read of this session. Send it, with a few lines about the car and the adapter, through the channel you received the build from.":
    "Один файл з оглядом, усіма захопленнями та читаннями цієї сесії. Надішліть його з кількома рядками про авто та адаптер тим каналом, яким отримали збірку.",
  Session: "Сесія",
  "Next: {title} — {hint}": "Далі: {title} — {hint}",
  "Everything recorded. Save the report and send it with the tester programme.":
    "Усе записано. Збережіть звіт і надішліть його за програмою тестування.",
  Done: "Виконано",
  Next: "Далі",
  "To do": "Попереду",
  optional: "необовʼязково",
  "Recorded in this session: {captures} capture(s), {reads} module read(s), {calibrations} calibration read(s).":
    "Записано в цій сесії: захоплень — {captures}, читань модулів — {reads}, читань калібрування — {calibrations}.",
  "Connect the adapter": "Підключити адаптер",
  "Detect the MongoosePro JLR and verify board communication.":
    "Знайти MongoosePro JLR і перевірити звʼязок із платою.",
  "Load the data library": "Завантажити бібліотеку даних",
  "The exported library folder given to you with the application.":
    "Папка експортованої бібліотеки, яку ви отримали разом із застосунком.",
  "Choose the vehicle": "Обрати автомобіль",
  "Programme and model years from the library; engine if known.":
    "Програма і модельні роки з бібліотеки; двигун, якщо відомий.",
  "Survey the modules": "Оглянути модулі",
  "Nothing is transmitted; every module is listed with its reach.":
    "Нічого не передається; кожен модуль показано з його досяжністю.",
  "Listen to a bus": "Прослухати шину",
  "Optional, zero-risk first contact: the adapter only listens.":
    "Необовʼязковий перший контакт без ризику: адаптер лише слухає.",
  "Read a module": "Прочитати модуль",
  "Fault codes or one identifier from a reachable module.":
    "Коди несправностей або один ідентифікатор із досяжного модуля.",
  "Save the session report": "Зберегти звіт сесії",
  "One file with everything recorded, for the tester programme.":
    "Один файл з усім записаним — для програми тестування.",
  "Adapter connected and board communication verified.":
    "Адаптер підключено, звʼязок із платою перевірено.",
  "Adapter detected.": "Адаптер виявлено.",
  "Adapter not detected.": "Адаптер не виявлено.",
  "Live vehicle status: not yet externally validated.":
    "Стан на живому авто: ще не підтверджено зовні.",

  // Vehicle card
  Vehicle: "Автомобіль",
  "Not chosen yet": "Ще не обрано",
  "Model years": "Модельні роки",
  Engine: "Двигун",
  "not stated": "не вказано",
  VIN: "VIN",
  "17 characters from the plate or the registration": "17 символів з таблички або техпаспорта",
  "Decoding…": "Розпізнавання…",
  "Decode VIN": "Розпізнати VIN",
  "Or choose the car from what the loaded data describes. A decoded VIN pre-selects it; confirm the engine.":
    "Або оберіть авто з того, що описують завантажені дані. Розпізнаний VIN обирає його наперед; підтвердьте двигун.",
  "Until a library is loaded, describe the vehicle as SDD names it.":
    "Поки бібліотеку не завантажено, опишіть автомобіль так, як його називає SDD.",
  Programme: "Програма",
  "Choose a programme": "Оберіть програму",
  Choose: "Оберіть",
  "Not stated": "Не вказано",
  "Model year": "Модельний рік",
  "SDD breakpoint marker": "Маркер SDD (breakpoint)",
  "as SDD names it": "як його називає SDD",
  "Working…": "Виконується…",
  "Survey modules": "Оглянути модулі",
  "Survey summary": "Підсумок огляду",
  "modules known": "модулів відомо",
  reachable: "досяжних",
  "on unverified routes": "на неперевірених маршрутах",
  "not reachable": "недосяжних",

  // Network map
  "Every module the data knows": "Кожен модуль, відомий даним",
  "Vehicle network": "Мережа автомобіля",
  "Also try unverified routes": "Також пробувати неперевірені маршрути",
  "Stop after this module ({done}/{total})": "Зупинити після цього модуля ({done}/{total})",
  "Check all modules ({count})": "Перевірити всі модулі ({count})",
  "Describe the vehicle and survey it: every module the loaded data associates with it appears here on its bus, with what the adapter can do about it. Nothing is transmitted by the survey.":
    "Опишіть автомобіль і огляньте його: кожен модуль, який завантажені дані повʼязують з ним, зʼявиться тут на своїй шині разом з тим, що адаптер може з ним зробити. Огляд нічого не передає.",
  " Connect and verify the adapter to read modules; the check sends one read-only fault-code request per module.":
    " Підключіть і перевірте адаптер, щоб читати модулі; перевірка надсилає кожному модулю один запит кодів несправностей (лише читання).",
  " The check sends one read-only fault-code request per module and marks what answered.":
    " Перевірка надсилає кожному модулю один запит кодів несправностей (лише читання) і позначає, хто відповів.",
  "{lane} modules": "Модулі шини {lane}",
  Legend: "Легенда",
  "documented route; can be read": "задокументований маршрут; можна читати",
  "route is a hypothesis; an answer confirms it": "маршрут — гіпотеза; відповідь її підтверджує",
  "not reachable from the adapter; the reason is shown": "недосяжний з адаптера; причину показано",
  "answered a read-only request": "відповів на запит читання",
  "no answer within the timeout": "немає відповіді за час очікування",
  "negative response or failed read": "негативна відповідь або невдале читання",
  "No bus": "Без шини",
  "bus not recorded in the data": "шина не записана в даних",
  "unverified: {route}": "неперевірено: {route}",
  "not bound: {reason}": "не привʼязано: {reason}",
  "not bound to the adapter": "не привʼязано до адаптера",
  "no adapter route": "маршруту адаптера немає",
  "pins {pins}": "контакти {pins}",

  // Node states
  "Reading…": "Читання…",
  "A read-only request is in flight.": "Запит читання надіслано, очікуємо відповідь.",
  Declined: "Відхилено",
  "The module answered with a negative response: {response}.":
    "Модуль відповів негативною відповіддю: {response}.",
  "1 fault code": "1 код несправності",
  "{count} fault codes": "Кодів несправностей: {count}",
  Answered: "Відповів",
  "Answered from {who} with {count} confirmed fault code(s).":
    "Відповідь від {who}: підтверджених кодів несправностей — {count}.",
  "Answered from {who}; no confirmed fault codes.":
    "Відповідь від {who}; підтверджених кодів несправностей немає.",
  "the module": "модуля",
  "No answer": "Немає відповіді",
  "No answer on the hypothesised route. Silence does not confirm it; the report records the attempt.":
    "Немає відповіді на гіпотетичному маршруті. Мовчання його не підтверджує; спробу записано у звіт.",
  "No answer within the timeout. The module may be absent, asleep, or on another bus.":
    "Немає відповіді за час очікування. Модуля може не бути, він може спати або бути на іншій шині.",
  Failed: "Помилка",
  "The read failed before an answer.": "Читання не вдалося до отримання відповіді.",
  Reachable: "Досяжний",
  "Documented route: {route}.": "Задокументований маршрут: {route}.",
  "Unverified route": "Неперевірений маршрут",
  "Route is a hypothesis: {route}. A read-only answer confirms it; silence refutes it.":
    "Маршрут — гіпотеза: {route}. Відповідь на запит читання підтверджує його; мовчання спростовує.",
  "Not on this vehicle": "Не на цьому авто",
  "The data does not associate this module with the described vehicle.":
    "Дані не повʼязують цей модуль з описаним автомобілем.",
  "Not reachable": "Недосяжний",
  "The data does not place this module on an adapter route.":
    "Дані не ставлять цей модуль на маршрут адаптера.",

  // Module details
  Module: "Модуль",
  "Choose a module": "Оберіть модуль",
  "Pick a module on the network map to see how it is reached, what it can be asked, and to read its fault codes or one identifier. Every request is read-only.":
    "Оберіть модуль на карті мережі, щоб побачити, як до нього дістатися, про що його можна запитати, і прочитати його коди несправностей або один ідентифікатор. Кожен запит — лише читання.",
  Read: "Прочитати",
  Bus: "Шина",
  "not recorded": "не записано",
  "Adapter route": "Маршрут адаптера",
  Addresses: "Адреси",
  Protocol: "Протокол",
  "Readable identifiers": "Читабельних ідентифікаторів",
  Operation: "Операція",
  "Confirmed fault codes": "Підтверджені коди несправностей",
  Identifier: "Ідентифікатор",
  "Choose an identifier": "Оберіть ідентифікатор",
  "Save read report": "Зберегти звіт читання",
  "This module's route is an unverified hypothesis. The request is read-only; an answer confirms the route, silence refutes it. Either way, save the report.":
    "Маршрут цього модуля — неперевірена гіпотеза. Запит лише читає; відповідь підтверджує маршрут, мовчання спростовує. У будь-якому разі збережіть звіт.",
  "Connect and verify MongoosePro JLR to enable reads.":
    "Підключіть і перевірте MongoosePro JLR, щоб читання стало доступним.",
  "Technical details": "Технічні деталі",
  Parameter: "Параметр",
  Value: "Значення",
  Note: "Примітка",
  "Fault code": "Код несправності",
  Description: "Опис",
  "Failure type": "Тип відмови",
  Status: "Стан",
  "No wording in the loaded data": "У завантажених даних немає опису",
  "generic wording": "загальний опис",
  "No confirmed fault codes reported.": "Підтверджених кодів несправностей не повідомлено.",
  "Exchange as it happened": "Обмін так, як він відбувся",
  Request: "Запит",
  Answer: "Відповідь",
  "after {count} response-pending": "після {count} відповідей «очікуйте»",
  Data: "Дані",

  // Tools: adapter
  Adapter: "Адаптер",
  "Adapter not detected": "Адаптер не виявлено",
  "Connect MongoosePro JLR by USB.": "Підключіть MongoosePro JLR через USB.",
  "Detect adapter": "Знайти адаптер",
  Disconnect: "Відʼєднати",
  Connected: "Підключено",
  Serial: "Серійний номер",
  "Not provided by OS": "ОС не повідомляє",
  Port: "Порт",
  Transport: "Транспорт",
  "Driver / backend": "Драйвер / бекенд",
  "Driver not reported": "Драйвер не повідомлено",
  "Board communication": "Звʼязок із платою",
  Verified: "Перевірено",
  "Board-info response": "Відповідь board-info",
  "Check the USB connection and try again.": "Перевірте USB-зʼєднання і спробуйте ще раз.",
  "{name} detected": "Виявлено {name}",
  Connecting: "Підключення",
  "Choose adapter": "Оберіть адаптер",
  "Select a COM port": "Оберіть COM-порт",
  "Connecting…": "Підключення…",
  Connect: "Підключити",
  "Detect again": "Шукати знову",
  // The bench (ADR-0020)
  "Connect the bench (virtual vehicle)": "Підʼєднати стенд (віртуальне авто)",
  "No adapter and no car needed: a virtual vehicle built from the library answers instead. Every value is synthetic.":
    "Без адаптера і без авто: замість них відповідає віртуальне авто, зібране з бібліотеки. Усі значення синтетичні.",
  "Virtual vehicle (bench)": "Віртуальне авто (стенд)",
  Bench: "Стенд",
  "Bench: virtual vehicle": "Стенд: віртуальне авто",
  "BENCH · virtual vehicle · synthetic data": "СТЕНД · віртуальне авто · дані синтетичні",
  "Every answer comes from the library, not from a car. Nothing from this session is written to disk.":
    "Кожна відповідь походить із бібліотеки, а не з авто. Нічого з цієї сесії не записується на диск.",
  "Show the report on screen": "Показати звіт на екрані",
  "Hide the report": "Сховати звіт",
  "Bench session: the report is shown on screen only and never saved. Bench data is illustrative, never evidence.":
    "Сесія на стенді: звіт лише показується на екрані й ніколи не зберігається. Дані стенда ілюстративні, а не доказові.",
  "Session report (bench, not saved)": "Звіт сесії (стенд, не збережено)",
  "Bench: nothing is saved.": "Стенд: нічого не зберігається.",
  "Bench: virtual vehicle from the library; every value is synthetic.":
    "Стенд: віртуальне авто з бібліотеки; усі значення синтетичні.",
  "Start a new session before switching between the bench and an adapter":
    "Перш ніж перемикатися між стендом і адаптером, почніть нову сесію",
  "A session is either on the bench or on a car, never both. The report, the survey and the reads of this session are dropped unless saved.":
    "Сесія або на стенді, або на авто — ніколи разом. Звіт, огляд і читання цієї сесії буде втрачено, якщо їх не збережено.",
  "Review the report on screen": "Переглянути звіт на екрані",
  "Bench: the report is shown on screen, never saved.": "Стенд: звіт показується на екрані й не зберігається.",
  Synthetic: "Синтетичні дані",

  // Tools: library
  Loaded: "Завантажено",
  "Partially loaded": "Завантажено частково",
  "Built-in data only": "Лише вбудовані дані",
  "Diagnostic data": "Діагностичні дані",
  "Data library": "Бібліотека даних",
  "The application reads vehicle definitions from a directory of exported data manifests. SDD itself is never required at run time.":
    "Застосунок читає описи автомобілів з папки експортованих маніфестів даних. Сам SDD під час роботи ніколи не потрібен.",
  "Directory of exported manifests": "Папка експортованих маніфестів",
  "Issued to {name} on {date}, copy {code}, valid until {until} ({days} days left). This copy is personal; every report carries it.":
    "Видано для {name} {date}, копія {code}, дійсна до {until} (лишилось днів: {days}). Ця копія особиста; кожен звіт її несе.",
  "This copy expires in {days} days; ask for a new one before then.":
    "Термін цієї копії спливає через {days} дн.; попросіть нову заздалегідь.",
  "This copy, issued to {name} until {until}, has expired. The library was not loaded; ask for a new copy.":
    "Термін цієї копії, виданої для {name} до {until}, минув. Бібліотеку не завантажено; попросіть нову копію.",
  "This folder carries no issue stamp. The application loads only a copy issued to a named person; ask for one.":
    "У цій папці немає штампа видачі. Застосунок завантажує лише копію, видану на ім'я; попросіть її.",
  "The stamp of this copy is not signed. The library was not loaded; ask for a new copy.":
    "Штамп цієї копії не підписано. Бібліотеку не завантажено; попросіть нову копію.",
  "The stamp's signature is not the owner's. The library was not loaded; ask for a new copy.":
    "Підпис штампа не належить власнику. Бібліотеку не завантажено; попросіть нову копію.",
  "The data does not match its stamp: issued to {name}, copy {code}. The library was not loaded; ask for a new copy.":
    "Дані не збігаються зі штампом: видано для {name}, копія {code}. Бібліотеку не завантажено; попросіть нову копію.",
  "The stamp file is missing; the data carries copy {code}. The library was not loaded; ask for a new copy.":
    "Файла штампа немає; дані несуть копію {code}. Бібліотеку не завантажено; попросіть нову копію.",
  "SDD reaches this sub-network through a gateway module with a routine command. This stage only reads and sends no commands, so these modules wait for stage 2; their addresses are known and nothing else is missing.":
    "SDD дістає до цієї підмережі через модуль-шлюз командою запуску процедури. Цей етап лише читає і команд не надсилає, тому ці модулі чекають на другий етап; їхні адреси відомі, і більше нічого не бракує.",
  "Behind a gateway: SDD opens it with a routine command, which this read-only stage does not send. Planned for stage 2; the address is known.":
    "За шлюзом: SDD відкриває його командою запуску процедури, якої цей етап лише читання не надсилає. Заплановано на другий етап; адреса відома.",
  "Connection": "З'єднання",
  "Choose folder…": "Обрати папку…",
  "Load library": "Завантажити бібліотеку",

  // Tools: capture
  "Listening…": "Слухаємо…",
  "Traffic heard": "Трафік чути",
  Silent: "Тиша",
  "Not run": "Не виконувалось",
  "Listen only": "Лише прослуховування",
  "Bus capture": "Захоплення шини",
  "Opens one CAN pair of the diagnostic connector in listen-only mode and records what the vehicle broadcasts. Nothing is transmitted; the adapter stays silent on the bus, so the vehicle cannot notice. The result says whether a live bus is on that pair; it cannot say which of the vehicle's buses it is.":
    "Відкриває одну CAN-пару діагностичного розʼєму в режимі лише прослуховування і записує, що передає автомобіль. Нічого не передається; адаптер мовчить на шині, тож автомобіль цього не помітить. Результат каже, чи є на цій парі жива шина; яка саме з шин автомобіля — сказати не може.",
  Pair: "Пара",
  "Seconds (1–15)": "Секунд (1–15)",
  Listen: "Слухати",
  "Save capture": "Зберегти захоплення",
  "Connect and verify MongoosePro JLR to enable listening.":
    "Підключіть і перевірте MongoosePro JLR, щоб прослуховування стало доступним.",
  "Capture did not complete": "Захоплення не завершилось",
  "Build: quote it when you report": "Збірка: назвіть її у звіті",
  Frames: "Кадрів",
  Identifiers: "Ідентифікаторів",
  "{distinct} distinct — {standard} standard, {extended} extended frames":
    "різних — {distinct}; стандартних кадрів — {standard}, розширених — {extended}",
  Notes: "Примітки",
  "Frame cap reached.": "Досягнуто межу кількості кадрів.",
  "{count} non-data packets dropped.": "Відкинуто пакетів без даних: {count}.",
  "{frames} in {seconds} s ({perSecond}/s)": "{frames} за {seconds} с ({perSecond}/с)",
  "the route's rate": "швидкості маршруту",
  "Traffic present on {route} (pins {pins}) at {rate}: about {perSecond} frames/s, {distinct} distinct identifiers. A live bus is on this pair. Which of the vehicle's buses it is cannot be told from listening alone.":
    "На {route} (піни {pins}) на {rate} є трафік: близько {perSecond} кадрів/с, різних ідентифікаторів — {distinct}. На цій парі жива шина. Яка саме з шин автомобіля — за самим прослуховуванням сказати не можна.",
  "No frames heard on {route} (pins {pins}) at {rate} in {seconds} s. Either this pair is silent at that rate — a diagnostic-only CAN behind a gateway carries nothing until a tester speaks — or the rate does not match, or nothing is connected. Listening alone cannot tell these apart.":
    "На {route} (піни {pins}) на {rate} за {seconds} с не почуто жодного кадру. Або ця пара мовчить на цій швидкості — суто діагностична CAN за шлюзом нічого не несе, доки тестер не заговорить, — або швидкість не та, або нічого не під'єднано. Саме прослуховування цього не розрізнить.",
  "The frame cap was reached before the time was up; the counts describe the captured part only.":
    "Межі кількості кадрів досягнуто раніше, ніж минув час; підрахунки описують лише захоплену частину.",
  "{count} non-data packets from the adapter were dropped.":
    "Відкинуто пакетів адаптера без даних: {count}.",
  Width: "Ширина",

  // Tools: calibration read (F8)
  Ready: "Готово",
  Complete: "Завершено",
  "Adapter required": "Потрібен адаптер",
  "Review the adapter connection and save the report for support.":
    "Перевірте підключення адаптера і збережіть звіт для підтримки.",
  Result: "Результат",
  "Read Calibration ID": "Прочитати Calibration ID",
  "Standard OBD-II read": "Стандартне читання OBD-II",
  "Engine calibration identifier": "Ідентифікатор калібрування двигуна",
  "Asks the engine module for its calibration identifier with a standard OBD-II request (mode 09) on hs-can, the same on every car with CAN diagnostics. Nothing is transmitted until you press the button.":
    "Запитує в блока керування двигуном ідентифікатор калібрування стандартним запитом OBD-II (режим 09) по hs-can, однаковим для всіх авто з CAN-діагностикою. Нічого не передається, доки ви не натиснете кнопку.",
  "Evidence-backed so far on {vehicle} only; on other cars the answer follows the standard and is not yet confirmed.":
    "Доказова база поки лише для {vehicle}; на інших авто відповідь очікується за стандартом і ще не підтверджена.",
  "Save Diagnostic Report": "Зберегти діагностичний звіт",
  "Connect and verify MongoosePro JLR to enable the read.":
    "Підключіть і перевірте MongoosePro JLR, щоб читання стало доступним.",
};

const ru: Record<string, string> = {
  // Header and session
  "Multi-platform vehicle diagnostics": "Кроссплатформенная диагностика автомобилей",
  "Simulator UI preview": "Предпросмотр интерфейса (симулятор)",
  "Development preview only — no Mongoose or vehicle communication.":
    "Только предпросмотр для разработки — без Mongoose и без связи с автомобилем.",
  "Adapter ready": "Адаптер готов",
  "Adapter connected, board unverified": "Адаптер подключён, плата не проверена",
  "Adapter found, not connected": "Адаптер найден, не подключён",
  "Adapter error": "Ошибка адаптера",
  "No adapter": "Адаптера нет",
  "Library: {records} records": "Библиотека: {records} записей",
  "Library failed to load": "Библиотека не загрузилась",
  "Library: built-in only": "Библиотека: только встроенные данные",
  "Saving…": "Сохранение…",
  "Save session report": "Сохранить отчёт сеанса",
  "New session": "Новый сеанс",
  "Start a new session?": "Начать новый сеанс?",
  "The report, the survey and the reads of this session are dropped unless saved. The adapter stays connected and the library stays loaded.":
    "Отчёт, обзор и чтения этого сеанса будут потеряны, если их не сохранить. Адаптер остаётся подключённым, библиотека загруженной.",
  "Start": "Начать",
  "Cancel": "Отмена",
  Language: "Язык",
  done: "выполнено",
  next: "следующий",
  "to do": "впереди",
  "One session, one report": "Один сеанс — один отчёт",
  "Preparation": "Подготовка",
  "Module network": "Сеть модулей",
  "Listening and standard OBD-II": "Прослушивание и стандартное OBD-II",
  "Session report": "Отчёт сеанса",
  "Step {list}": "Шаг {list}",
  "Steps {list}": "Шаги {list}",
  "Go to step {index}": "Перейти к шагу {index}",
  "One file with the survey, every capture and every read of this session. Send it, with a few lines about the car and the adapter, through the channel you received the build from.":
    "Один файл с обзором, всеми захватами и чтениями этого сеанса. Отправьте его с несколькими строками об авто и адаптере тем каналом, которым получили сборку.",
  Session: "Сеанс",
  "Next: {title} — {hint}": "Далее: {title} — {hint}",
  "Everything recorded. Save the report and send it with the tester programme.":
    "Всё записано. Сохраните отчёт и отправьте его по программе тестирования.",
  Done: "Выполнено",
  Next: "Далее",
  "To do": "Впереди",
  optional: "необязательно",
  "Recorded in this session: {captures} capture(s), {reads} module read(s), {calibrations} calibration read(s).":
    "Записано в этом сеансе: захватов — {captures}, чтений модулей — {reads}, чтений калибровки — {calibrations}.",
  "Connect the adapter": "Подключить адаптер",
  "Detect the MongoosePro JLR and verify board communication.":
    "Найти MongoosePro JLR и проверить связь с платой.",
  "Load the data library": "Загрузить библиотеку данных",
  "The exported library folder given to you with the application.":
    "Папка экспортированной библиотеки, полученная вместе с приложением.",
  "Choose the vehicle": "Выбрать автомобиль",
  "Programme and model years from the library; engine if known.":
    "Программа и модельные годы из библиотеки; двигатель, если известен.",
  "Survey the modules": "Осмотреть модули",
  "Nothing is transmitted; every module is listed with its reach.":
    "Ничего не передаётся; каждый модуль показан с его достижимостью.",
  "Listen to a bus": "Прослушать шину",
  "Optional, zero-risk first contact: the adapter only listens.":
    "Необязательный первый контакт без риска: адаптер только слушает.",
  "Read a module": "Прочитать модуль",
  "Fault codes or one identifier from a reachable module.":
    "Коды неисправностей или один идентификатор из достижимого модуля.",
  "Save the session report": "Сохранить отчёт сеанса",
  "One file with everything recorded, for the tester programme.":
    "Один файл со всем записанным — для программы тестирования.",
  "Adapter connected and board communication verified.":
    "Адаптер подключён, связь с платой проверена.",
  "Adapter detected.": "Адаптер обнаружен.",
  "Adapter not detected.": "Адаптер не обнаружен.",
  "Live vehicle status: not yet externally validated.":
    "Состояние на живом автомобиле: ещё не подтверждено извне.",

  // Vehicle card
  Vehicle: "Автомобиль",
  "Not chosen yet": "Ещё не выбран",
  "Model years": "Модельные годы",
  Engine: "Двигатель",
  "not stated": "не указан",
  VIN: "VIN",
  "17 characters from the plate or the registration": "17 символов с таблички или из техпаспорта",
  "Decoding…": "Распознавание…",
  "Decode VIN": "Распознать VIN",
  "Or choose the car from what the loaded data describes. A decoded VIN pre-selects it; confirm the engine.":
    "Или выберите автомобиль из того, что описывают загруженные данные. Распознанный VIN выбирает его заранее; подтвердите двигатель.",
  "Until a library is loaded, describe the vehicle as SDD names it.":
    "Пока библиотека не загружена, опишите автомобиль так, как его называет SDD.",
  Programme: "Программа",
  "Choose a programme": "Выберите программу",
  Choose: "Выберите",
  "Not stated": "Не указан",
  "Model year": "Модельный год",
  "SDD breakpoint marker": "Маркер SDD (breakpoint)",
  "as SDD names it": "как его называет SDD",
  "Working…": "Выполняется…",
  "Survey modules": "Осмотреть модули",
  "Survey summary": "Итог осмотра",
  "modules known": "модулей известно",
  reachable: "достижимых",
  "on unverified routes": "на непроверенных маршрутах",
  "not reachable": "недостижимых",

  // Network map
  "Every module the data knows": "Каждый модуль, известный данным",
  "Vehicle network": "Сеть автомобиля",
  "Also try unverified routes": "Также пробовать непроверенные маршруты",
  "Stop after this module ({done}/{total})": "Остановить после этого модуля ({done}/{total})",
  "Check all modules ({count})": "Проверить все модули ({count})",
  "Describe the vehicle and survey it: every module the loaded data associates with it appears here on its bus, with what the adapter can do about it. Nothing is transmitted by the survey.":
    "Опишите автомобиль и осмотрите его: каждый модуль, который загруженные данные связывают с ним, появится здесь на своей шине вместе с тем, что адаптер может с ним сделать. Осмотр ничего не передаёт.",
  " Connect and verify the adapter to read modules; the check sends one read-only fault-code request per module.":
    " Подключите и проверьте адаптер, чтобы читать модули; проверка отправляет каждому модулю один запрос кодов неисправностей (только чтение).",
  " The check sends one read-only fault-code request per module and marks what answered.":
    " Проверка отправляет каждому модулю один запрос кодов неисправностей (только чтение) и отмечает, кто ответил.",
  "{lane} modules": "Модули шины {lane}",
  Legend: "Легенда",
  "documented route; can be read": "задокументированный маршрут; можно читать",
  "route is a hypothesis; an answer confirms it": "маршрут — гипотеза; ответ её подтверждает",
  "not reachable from the adapter; the reason is shown": "недостижим с адаптера; причина показана",
  "answered a read-only request": "ответил на запрос чтения",
  "no answer within the timeout": "нет ответа за время ожидания",
  "negative response or failed read": "отрицательный ответ или неудачное чтение",
  "No bus": "Без шины",
  "bus not recorded in the data": "шина не записана в данных",
  "unverified: {route}": "не проверено: {route}",
  "not bound: {reason}": "не привязано: {reason}",
  "not bound to the adapter": "не привязано к адаптеру",
  "no adapter route": "маршрута адаптера нет",
  "pins {pins}": "контакты {pins}",

  // Node states
  "Reading…": "Чтение…",
  "A read-only request is in flight.": "Запрос чтения отправлен, ждём ответ.",
  Declined: "Отклонено",
  "The module answered with a negative response: {response}.":
    "Модуль ответил отрицательным ответом: {response}.",
  "1 fault code": "1 код неисправности",
  "{count} fault codes": "Кодов неисправностей: {count}",
  Answered: "Ответил",
  "Answered from {who} with {count} confirmed fault code(s).":
    "Ответ от {who}: подтверждённых кодов неисправностей — {count}.",
  "Answered from {who}; no confirmed fault codes.":
    "Ответ от {who}; подтверждённых кодов неисправностей нет.",
  "the module": "модуля",
  "No answer": "Нет ответа",
  "No answer on the hypothesised route. Silence does not confirm it; the report records the attempt.":
    "Нет ответа на гипотетическом маршруте. Молчание его не подтверждает; попытка записана в отчёт.",
  "No answer within the timeout. The module may be absent, asleep, or on another bus.":
    "Нет ответа за время ожидания. Модуля может не быть, он может спать или быть на другой шине.",
  Failed: "Ошибка",
  "The read failed before an answer.": "Чтение не удалось до получения ответа.",
  Reachable: "Достижим",
  "Documented route: {route}.": "Задокументированный маршрут: {route}.",
  "Unverified route": "Непроверенный маршрут",
  "Route is a hypothesis: {route}. A read-only answer confirms it; silence refutes it.":
    "Маршрут — гипотеза: {route}. Ответ на запрос чтения подтверждает его; молчание опровергает.",
  "Not on this vehicle": "Не на этом автомобиле",
  "The data does not associate this module with the described vehicle.":
    "Данные не связывают этот модуль с описанным автомобилем.",
  "Not reachable": "Недостижим",
  "The data does not place this module on an adapter route.":
    "Данные не ставят этот модуль на маршрут адаптера.",

  // Module details
  Module: "Модуль",
  "Choose a module": "Выберите модуль",
  "Pick a module on the network map to see how it is reached, what it can be asked, and to read its fault codes or one identifier. Every request is read-only.":
    "Выберите модуль на карте сети, чтобы увидеть, как до него добраться, о чём его можно спросить, и прочитать его коды неисправностей или один идентификатор. Каждый запрос — только чтение.",
  Read: "Прочитать",
  Bus: "Шина",
  "not recorded": "не записано",
  "Adapter route": "Маршрут адаптера",
  Addresses: "Адреса",
  Protocol: "Протокол",
  "Readable identifiers": "Читаемых идентификаторов",
  Operation: "Операция",
  "Confirmed fault codes": "Подтверждённые коды неисправностей",
  Identifier: "Идентификатор",
  "Choose an identifier": "Выберите идентификатор",
  "Save read report": "Сохранить отчёт чтения",
  "This module's route is an unverified hypothesis. The request is read-only; an answer confirms the route, silence refutes it. Either way, save the report.":
    "Маршрут этого модуля — непроверенная гипотеза. Запрос только читает; ответ подтверждает маршрут, молчание опровергает. В любом случае сохраните отчёт.",
  "Connect and verify MongoosePro JLR to enable reads.":
    "Подключите и проверьте MongoosePro JLR, чтобы чтение стало доступным.",
  "Technical details": "Технические подробности",
  Parameter: "Параметр",
  Value: "Значение",
  Note: "Примечание",
  "Fault code": "Код неисправности",
  Description: "Описание",
  "Failure type": "Тип отказа",
  Status: "Состояние",
  "No wording in the loaded data": "В загруженных данных нет описания",
  "generic wording": "общее описание",
  "No confirmed fault codes reported.": "Подтверждённых кодов неисправностей не сообщено.",
  "Exchange as it happened": "Обмен так, как он прошёл",
  Request: "Запрос",
  Answer: "Ответ",
  "after {count} response-pending": "после {count} ответов «ожидайте»",
  Data: "Данные",

  // Tools: adapter
  Adapter: "Адаптер",
  "Adapter not detected": "Адаптер не обнаружен",
  "Connect MongoosePro JLR by USB.": "Подключите MongoosePro JLR по USB.",
  "Detect adapter": "Найти адаптер",
  Disconnect: "Отключить",
  Connected: "Подключено",
  Serial: "Серийный номер",
  "Not provided by OS": "ОС не сообщает",
  Port: "Порт",
  Transport: "Транспорт",
  "Driver / backend": "Драйвер / бэкенд",
  "Driver not reported": "Драйвер не сообщён",
  "Board communication": "Связь с платой",
  Verified: "Проверено",
  "Board-info response": "Ответ board-info",
  "Check the USB connection and try again.": "Проверьте USB-соединение и попробуйте снова.",
  "{name} detected": "Обнаружен {name}",
  Connecting: "Подключение",
  "Choose adapter": "Выберите адаптер",
  "Select a COM port": "Выберите COM-порт",
  "Connecting…": "Подключение…",
  Connect: "Подключить",
  "Detect again": "Искать снова",
  // The bench (ADR-0020)
  "Connect the bench (virtual vehicle)": "Подключить стенд (виртуальный автомобиль)",
  "No adapter and no car needed: a virtual vehicle built from the library answers instead. Every value is synthetic.":
    "Без адаптера и без автомобиля: вместо них отвечает виртуальный автомобиль, собранный из библиотеки. Все значения синтетические.",
  "Virtual vehicle (bench)": "Виртуальный автомобиль (стенд)",
  Bench: "Стенд",
  "Bench: virtual vehicle": "Стенд: виртуальный автомобиль",
  "BENCH · virtual vehicle · synthetic data": "СТЕНД · виртуальный автомобиль · данные синтетические",
  "Every answer comes from the library, not from a car. Nothing from this session is written to disk.":
    "Каждый ответ приходит из библиотеки, а не из автомобиля. Ничего из этого сеанса не записывается на диск.",
  "Show the report on screen": "Показать отчёт на экране",
  "Hide the report": "Скрыть отчёт",
  "Bench session: the report is shown on screen only and never saved. Bench data is illustrative, never evidence.":
    "Сеанс на стенде: отчёт только показывается на экране и никогда не сохраняется. Данные стенда иллюстративные, а не доказательные.",
  "Session report (bench, not saved)": "Отчёт сеанса (стенд, не сохранён)",
  "Bench: nothing is saved.": "Стенд: ничего не сохраняется.",
  "Bench: virtual vehicle from the library; every value is synthetic.":
    "Стенд: виртуальный автомобиль из библиотеки; все значения синтетические.",
  "Start a new session before switching between the bench and an adapter":
    "Прежде чем переключаться между стендом и адаптером, начните новый сеанс",
  "A session is either on the bench or on a car, never both. The report, the survey and the reads of this session are dropped unless saved.":
    "Сеанс либо на стенде, либо на автомобиле — никогда вместе. Отчёт, обзор и чтения этого сеанса будут потеряны, если их не сохранить.",
  "Review the report on screen": "Просмотреть отчёт на экране",
  "Bench: the report is shown on screen, never saved.": "Стенд: отчёт показывается на экране и не сохраняется.",
  Synthetic: "Синтетические данные",

  // Tools: library
  Loaded: "Загружено",
  "Partially loaded": "Загружено частично",
  "Built-in data only": "Только встроенные данные",
  "Diagnostic data": "Диагностические данные",
  "Data library": "Библиотека данных",
  "The application reads vehicle definitions from a directory of exported data manifests. SDD itself is never required at run time.":
    "Приложение читает описания автомобилей из папки экспортированных манифестов данных. Сам SDD во время работы никогда не нужен.",
  "Directory of exported manifests": "Папка экспортированных манифестов",
  "Issued to {name} on {date}, copy {code}, valid until {until} ({days} days left). This copy is personal; every report carries it.":
    "Выдано для {name} {date}, копия {code}, действительна до {until} (осталось дней: {days}). Эта копия личная; каждый отчёт её несёт.",
  "This copy expires in {days} days; ask for a new one before then.":
    "Срок этой копии истекает через {days} дн.; попросите новую заранее.",
  "This copy, issued to {name} until {until}, has expired. The library was not loaded; ask for a new copy.":
    "Срок этой копии, выданной для {name} до {until}, истёк. Библиотека не загружена; попросите новую копию.",
  "This folder carries no issue stamp. The application loads only a copy issued to a named person; ask for one.":
    "В этой папке нет штампа выдачи. Приложение загружает только копию, выданную на имя; попросите её.",
  "The stamp of this copy is not signed. The library was not loaded; ask for a new copy.":
    "Штамп этой копии не подписан. Библиотека не загружена; попросите новую копию.",
  "The stamp's signature is not the owner's. The library was not loaded; ask for a new copy.":
    "Подпись штампа не принадлежит владельцу. Библиотека не загружена; попросите новую копию.",
  "The data does not match its stamp: issued to {name}, copy {code}. The library was not loaded; ask for a new copy.":
    "Данные не совпадают со штампом: выдано для {name}, копия {code}. Библиотека не загружена; попросите новую копию.",
  "The stamp file is missing; the data carries copy {code}. The library was not loaded; ask for a new copy.":
    "Файла штампа нет; данные несут копию {code}. Библиотека не загружена; попросите новую копию.",
  "SDD reaches this sub-network through a gateway module with a routine command. This stage only reads and sends no commands, so these modules wait for stage 2; their addresses are known and nothing else is missing.":
    "SDD достигает этой подсети через модуль-шлюз командой запуска процедуры. Этот этап только читает и команд не посылает, поэтому эти модули ждут второго этапа; их адреса известны, и больше ничего не недостаёт.",
  "Behind a gateway: SDD opens it with a routine command, which this read-only stage does not send. Planned for stage 2; the address is known.":
    "За шлюзом: SDD открывает его командой запуска процедуры, которую этот этап только чтения не посылает. Запланировано на второй этап; адрес известен.",
  "Connection": "Подключение",
  "Choose folder…": "Выбрать папку…",
  "Load library": "Загрузить библиотеку",

  // Tools: capture
  "Listening…": "Слушаем…",
  "Traffic heard": "Трафик слышен",
  Silent: "Тишина",
  "Not run": "Не выполнялось",
  "Listen only": "Только прослушивание",
  "Bus capture": "Захват шины",
  "Opens one CAN pair of the diagnostic connector in listen-only mode and records what the vehicle broadcasts. Nothing is transmitted; the adapter stays silent on the bus, so the vehicle cannot notice. The result says whether a live bus is on that pair; it cannot say which of the vehicle's buses it is.":
    "Открывает одну CAN-пару диагностического разъёма в режиме только прослушивания и записывает, что передаёт автомобиль. Ничего не передаётся; адаптер молчит на шине, поэтому автомобиль этого не заметит. Результат говорит, есть ли на этой паре живая шина; какая именно из шин автомобиля — сказать не может.",
  Pair: "Пара",
  "Seconds (1–15)": "Секунд (1–15)",
  Listen: "Слушать",
  "Save capture": "Сохранить захват",
  "Connect and verify MongoosePro JLR to enable listening.":
    "Подключите и проверьте MongoosePro JLR, чтобы прослушивание стало доступным.",
  "Capture did not complete": "Захват не завершился",
  "Build: quote it when you report": "Сборка: назовите её в отчёте",
  Frames: "Кадров",
  Identifiers: "Идентификаторов",
  "{distinct} distinct — {standard} standard, {extended} extended frames":
    "разных — {distinct}; стандартных кадров — {standard}, расширенных — {extended}",
  Notes: "Примечания",
  "Frame cap reached.": "Достигнут предел количества кадров.",
  "{count} non-data packets dropped.": "Отброшено пакетов без данных: {count}.",
  "{frames} in {seconds} s ({perSecond}/s)": "{frames} за {seconds} с ({perSecond}/с)",
  "the route's rate": "скорости маршрута",
  "Traffic present on {route} (pins {pins}) at {rate}: about {perSecond} frames/s, {distinct} distinct identifiers. A live bus is on this pair. Which of the vehicle's buses it is cannot be told from listening alone.":
    "На {route} (пины {pins}) на {rate} есть трафик: около {perSecond} кадров/с, разных идентификаторов — {distinct}. На этой паре живая шина. Какая именно из шин автомобиля — по одному прослушиванию сказать нельзя.",
  "No frames heard on {route} (pins {pins}) at {rate} in {seconds} s. Either this pair is silent at that rate — a diagnostic-only CAN behind a gateway carries nothing until a tester speaks — or the rate does not match, or nothing is connected. Listening alone cannot tell these apart.":
    "На {route} (пины {pins}) на {rate} за {seconds} с не услышано ни одного кадра. Либо эта пара молчит на этой скорости — чисто диагностическая CAN за шлюзом ничего не несёт, пока тестер не заговорит, — либо скорость не та, либо ничего не подключено. Одно прослушивание этого не различит.",
  "The frame cap was reached before the time was up; the counts describe the captured part only.":
    "Предел количества кадров достигнут раньше, чем истекло время; подсчёты описывают только захваченную часть.",
  "{count} non-data packets from the adapter were dropped.":
    "Отброшено пакетов адаптера без данных: {count}.",
  Width: "Ширина",

  // Tools: calibration read (F8)
  Ready: "Готово",
  Complete: "Завершено",
  "Adapter required": "Нужен адаптер",
  "Review the adapter connection and save the report for support.":
    "Проверьте подключение адаптера и сохраните отчёт для поддержки.",
  Result: "Результат",
  "Read Calibration ID": "Прочитать Calibration ID",
  "Standard OBD-II read": "Стандартное чтение OBD-II",
  "Engine calibration identifier": "Идентификатор калибровки двигателя",
  "Asks the engine module for its calibration identifier with a standard OBD-II request (mode 09) on hs-can, the same on every car with CAN diagnostics. Nothing is transmitted until you press the button.":
    "Запрашивает у блока управления двигателем идентификатор калибровки стандартным запросом OBD-II (режим 09) по hs-can, одинаковым для всех авто с CAN-диагностикой. Ничего не передаётся, пока вы не нажмёте кнопку.",
  "Evidence-backed so far on {vehicle} only; on other cars the answer follows the standard and is not yet confirmed.":
    "Доказательная база пока только для {vehicle}; на других авто ответ ожидается по стандарту и ещё не подтверждён.",
  "Save Diagnostic Report": "Сохранить диагностический отчёт",
  "Connect and verify MongoosePro JLR to enable the read.":
    "Подключите и проверьте MongoosePro JLR, чтобы чтение стало доступным.",
};

const dictionaries: Record<Language, Record<string, string>> = { en: {}, ru, uk };

let current: Language = "en";

export function readStoredLanguage(): Language {
  try {
    const stored = window.localStorage.getItem(LANGUAGE_STORAGE_KEY);
    return stored === "uk" || stored === "ru" ? stored : "en";
  } catch {
    return "en";
  }
}

export function setCurrentLanguage(language: Language) {
  current = language;
  try {
    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
  } catch {
    // A browser that refuses storage still gets the language for this session.
  }
}

export function currentLanguage(): Language {
  return current;
}

/** Translate an English string into the current language, filling `{name}` placeholders. */
export function t(text: string, params?: Record<string, string | number>): string {
  const translated = dictionaries[current][text] ?? text;
  if (params === undefined) return translated;
  return translated.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

export const LanguageContext = createContext<Language>("en");

/** The current language, for components that must re-render when it changes. */
export function useLanguage(): Language {
  return useContext(LanguageContext);
}
