# bounce-clock

Пиксельные часы для терминала: DVD-отскок, статика и ручное управление.
Рисуются только символами `▀▄█`, поэтому выглядят одинаково везде, даже в Linux VT.

Claude и идея Даниила Кочубейника.

## Установка

### Windows

```powershell
winget install Kochubeynik.BounceClock
```

Или без winget, одной строкой в PowerShell:

```powershell
irm https://raw.githubusercontent.com/ЛОГИН/bounce-clock/main/install.ps1 | iex
```

Скрипт кладёт `bounce-clock.exe` в `%LOCALAPPDATA%\Programs\bounce-clock` и добавляет папку в PATH.
Лучше всего смотрится в Windows Terminal.

### Linux

Любой дистрибутив, одной строкой:

```sh
curl -fsSL https://raw.githubusercontent.com/ЛОГИН/bounce-clock/main/install.sh | sh
```

Бинарник статический (musl), зависимостей нет. Ставится в `~/.local/bin`, под root — в `/usr/local/bin`.

Arch Linux (AUR):

```sh
yay -S bounce-clock-bin   # готовая сборка
yay -S bounce-clock       # сборка из исходников
```

Debian / Ubuntu: скачайте `.deb` со страницы [Releases](https://github.com/ЛОГИН/bounce-clock/releases/latest) и

```sh
sudo apt install ./bounce-clock_*.deb
```

### Через cargo (любая ОС)

```sh
cargo install --git https://github.com/ЛОГИН/bounce-clock --locked
```

### Удаление

```sh
curl -fsSL https://raw.githubusercontent.com/ЛОГИН/bounce-clock/main/install.sh | sh -s -- --uninstall
```

```powershell
winget uninstall Kochubeynik.BounceClock
```

## Запуск

```sh
bounce-clock                    # по умолчанию — отскок
bounce-clock --mode static      # стоит на месте
bounce-clock --no-seconds       # только часы и минуты
bounce-clock --print-config > ~/.config/bounce-clock/config.toml
bounce-clock --help
```

| Клавиша | Действие |
|---|---|
| пробел, m / 1 2 3 | следующий режим / bounce, static, manual |
| стрелки, Shift+стрелки | толкнуть / сдвиг на 1 пиксель |
| c, a, z | в центр, прицелиться в угол, замедление |
| t, p | стиль смены цифр, профиль |
| Tab | окно настроек (w — сохранить) |
| ?, q | справка, выход (повторное q — сразу) |

## Конфиг

`~/.config/bounce-clock/config.toml`, на Windows `%APPDATA%\bounce-clock\config.toml`.
Изменения подхватываются на лету. Полный пример со всеми параметрами, профилями,
расписанием и событиями печатает `bounce-clock --print-config`.

## Сборка из исходников

```sh
git clone https://github.com/ЛОГИН/bounce-clock
cd bounce-clock
cargo build --release
./target/release/bounce-clock
```

Устройство программы описано в [DESIGN.md](DESIGN.md), выпуск версий — в [RELEASING.md](RELEASING.md).

## Лицензия

MIT
