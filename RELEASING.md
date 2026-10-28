# Выпуск версии

## Один раз: первая публикация

### 1. Репозиторий

```sh
grep -rlw danikforcode --exclude-dir=target --exclude=RELEASING.md . | xargs sed -i 's/\bUSER\b/danikforcode/g'
```

```sh
git init
git add .
git commit -m "bounce-clock 0.0.2"
git branch -M main
git remote add origin git@github.com:danikforcode/bounce-clock.git
git push -u origin main
```

Репозиторий должен быть публичным, иначе скрипты установки не скачают релиз.

### 2. Релиз

```sh
git tag v0.0.2
git push origin v0.0.2
```

GitHub Actions (`.github/workflows/release.yml`) соберёт за ~5 минут:

| Файл | Для чего |
|---|---|
| `bounce-clock-x86_64-unknown-linux-musl.tar.gz` | Linux x86_64, install.sh, AUR -bin |
| `bounce-clock-aarch64-unknown-linux-musl.tar.gz` | Linux ARM (Raspberry Pi и т. п.) |
| `bounce-clock_0.0.2-1_amd64.deb`, `..._arm64.deb` | Debian / Ubuntu |
| `bounce-clock-x86_64-pc-windows-msvc.zip` | Windows x64, winget |
| `bounce-clock-aarch64-pc-windows-msvc.zip` | Windows ARM |
| `SHA256SUMS` и `*.sha256` | контрольные суммы |

После этого уже работают `install.sh`, `install.ps1` и `cargo install --git`.

### 3. winget

Пакеты winget живут в общем репозитории `microsoft/winget-pkgs`; туда отправляется PR,
его проверяют автоматически и вручную (обычно от нескольких часов до пары дней).

Проще всего через wingetcreate — он сам посчитает хэши:

```powershell
winget install Microsoft.WingetCreate
wingetcreate new https://github.com/danikforcode/bounce-clock/releases/download/v0.0.2/bounce-clock-x86_64-pc-windows-msvc.zip https://github.com/danikforcode/bounce-clock/releases/download/v0.0.2/bounce-clock-aarch64-pc-windows-msvc.zip
```

На вопросы отвечайте по готовым манифестам из `packaging/winget/` (идентификатор `Kochubeynik.BounceClock`,
тип portable, команда `bounce-clock`), в конце согласитесь отправить PR.

Вариант вручную: вставьте хэши из `SHA256SUMS` в `Kochubeynik.BounceClock.installer.yaml`, затем

```powershell
winget validate packaging\winget\manifests\k\Kochubeynik\BounceClock\0.0.2
winget settings --enable LocalManifestFiles     # от администратора
winget install --manifest packaging\winget\manifests\k\Kochubeynik\BounceClock\0.0.2
```

и скопируйте папку `manifests/k/Kochubeynik/BounceClock/0.0.2` в свой форк `winget-pkgs` с PR.

Пока PR не принят, `winget install Kochubeynik.BounceClock` не найдёт пакет — пользуйтесь `install.ps1`.

### 4. AUR

Нужен аккаунт на aur.archlinux.org с SSH-ключом. Для каждого из двух пакетов:

```sh
git clone ssh://aur@aur.archlinux.org/bounce-clock-bin.git aur-bin
cp packaging/aur/bounce-clock-bin/PKGBUILD aur-bin/
cd aur-bin
updpkgsums                         # заменит SKIP на настоящие хэши
makepkg -si                        # проверить, что собирается и ставится
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO && git commit -m "0.0.2" && git push
```

То же для `bounce-clock` (сборка из исходников). В PKGBUILD поменяйте `<почта>` на свою.

## Следующие версии

1. Поднимите `version` в `Cargo.toml` (и строку версии в `config.rs` → `example()`), закоммитьте.
2. `git tag vX.Y.Z && git push origin vX.Y.Z` — релиз соберётся сам.
3. winget: если в Secrets добавлен `WINGET_TOKEN` (classic PAT с `public_repo`, нужен форк `winget-pkgs`),
   workflow `winget.yml` сам отправит PR с новой версией. Иначе: `wingetcreate update Kochubeynik.BounceClock -v X.Y.Z -u <url x64> <url arm64> --submit`.
4. AUR: в обоих PKGBUILD поменяйте `pkgver`, `pkgrel=1`, затем `updpkgsums`, `.SRCINFO`, push.
