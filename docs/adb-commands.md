# ADB Commands Used

This document lists every ADB command invoked by Android Utility, the expected
and possible outputs, and the minimum supported Android version for each device
command.

Every new ADB command must be added to this document when it is introduced. Do
not merge a command without its purpose, possible outputs, and minimum supported
Android version.

Source of truth: `src/adb/client.rs`. Parsers and validators live under
`src/adb/parse/`.

## Invocation Rules

- Commands are passed as argument arrays. The application does not build shell
  command strings.
- Device commands run as `adb -s <serial> shell <arguments...>`.
- Short commands use a 20-second timeout. APK pulls use a 120-second timeout.
- A timeout, non-zero exit code, or malformed output is reported as a failure.
- State-changing commands call `adb -s <serial> get-state` first.
- Some device builds backport, remove, or change commands. The application
  validates exit status and output instead of assuming success.

## Quick Reference

| Command | Purpose | Minimum Android | Output validation |
|---|---|---|---|
| `adb devices -l` | List connected devices and states. | Host command; no device API requirement. | Exit code must be 0. |
| `adb -s <serial> get-state` | Confirm a device is ready before a state change. | Any ADB-capable device. | Stdout must be `device`. |
| `adb -s <serial> shell pm list packages -f` | List packages with APK paths. | Android 1.0 (API 1). Flag support varies on old vendor builds. | Rejects reported `failure` or `exception`. |
| `adb -s <serial> shell pm list packages -s` | List system packages. | Android 1.0 (API 1). | Rejects reported `failure` or `exception`. |
| `adb -s <serial> shell pm list packages -d` | List disabled packages. | Android 1.0 (API 1). | Rejects reported `failure` or `exception`. |
| `adb -s <serial> shell top -b -n 1 -o PID,%CPU,RES,TIME+,ARGS -s 2` | Read one process snapshot. | Android 8.0 (API 26). | Requires a parseable process table. |
| `adb -s <serial> shell dumpsys deviceidle whitelist` | Read the Doze whitelist. | Android 7.0 (API 24). | Exit code must be 0. |
| `adb -s <serial> shell dumpsys deviceidle whitelist +<package>` | Add a package to the Doze whitelist. | Android 7.0 (API 24). | Exit code must be 0 and stdout must not report an unknown package. |
| `adb -s <serial> shell dumpsys deviceidle whitelist -<package>` | Remove a package from the Doze whitelist. | Android 7.0 (API 24). | Same as the add command. |
| `adb -s <serial> shell cmd appops query-op RUN_ANY_IN_BACKGROUND ignore` | List packages whose background execution is ignored. | Android 9.0 (API 28). | Exit code must be 0; stderr must not report an error or exception. |
| `adb -s <serial> shell cmd appops set <package> RUN_ANY_IN_BACKGROUND allow` | Allow background execution. | Android 9.0 (API 28). | Same as the query command. |
| `adb -s <serial> shell cmd appops set <package> RUN_ANY_IN_BACKGROUND ignore` | Restrict background execution. | Android 9.0 (API 28). | Same as the query command. |
| `adb -s <serial> shell pm disable-user --user 0 <package>` | Disable a package for the current user. | Android 4.2 (API 17) for `--user`. | Rejects reported `failure` or `exception`. |
| `adb -s <serial> shell pm enable --user 0 <package>` | Enable a package for the current user. | Android 4.2 (API 17) for `--user`. | Rejects reported `failure` or `exception`. |
| `adb -s <serial> shell pm uninstall --user 0 <package>` | Uninstall a package for the current user. | Android 5.0 (API 21) for reliable `--user` removal. Some Android 4.2-4.4 builds accept it. | Stdout must be `Success` and the command must not report a failure. |
| `adb -s <serial> pull <remote-apk> <local-apk>` | Copy an APK to read its application label. | Host command; no device API requirement. | Pull must exit with code 0. |

## Device Discovery

### `adb devices -l`

Lists devices known to the ADB server with transport details.

Example output:

```text
List of devices attached
emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:1
R58M12345              offline
ABC123                 unauthorized
XYZ                    no permissions (user in plugdev group); see [http://developer.android.com]
```

Possible outputs:

- One or more device lines.
- `List of devices attached` with no device lines.
- `adb: failed to check server version: ...`
- `cannot connect to daemon`

The parser ignores the header and lines beginning with `*`. It reads the serial,
state, and optional `model:` field.

Minimum Android version: not applicable. This is an ADB host command. The `-l`
flag requires a modern Android SDK Platform-Tools release.

## Device Readiness

### `adb -s <serial> get-state`

Checks that the selected device is ready before a state-changing command.

Expected output:

```text
device
```

Possible outputs:

```text
offline
unauthorized
unknown
error: device 'SERIAL' not found
```

The command is treated as successful only when it exits with code 0 and stdout
is exactly `device`.

Minimum Android version: any ADB-capable device.

## Package Listing

### `adb -s <serial> shell pm list packages -f`

Lists packages and the APK paths that contain them.

Example output:

```text
package:/data/app/~~abc==/com.example-1/base.apk=com.example
package:/system/priv-app/Settings/Settings.apk=com.android.settings
package:com.legacy.package
```

Variants:

- The APK path may be omitted.
- A package may appear more than once, for example when split APKs are listed.
- The path and package are separated by `=`.
- Failures can print diagnostics on stdout or stderr.

The parser keeps only lines beginning with `package:`, splits off the package
name, validates it with `PackageName::parse`, and merges duplicates while
preferring `/base.apk`.

Minimum Android version: Android 1.0 (API 1) for `pm list packages`. Support for
the `-f` flag varies on very old or vendor-modified builds.

### `adb -s <serial> shell pm list packages -s`

Lists packages that belong to the system image.

Example output:

```text
package:com.android.settings
package:com.android.shell
package:com.google.android.gms
```

The parser extracts the package names and uses them to mark entries returned by
the `-f` listing as system applications.

Minimum Android version: Android 1.0 (API 1). The `-s` flag is not available on
every vendor build.

### `adb -s <serial> shell pm list packages -d`

Lists packages that are disabled for the current user.

Example output:

```text
package:com.example.old
package:com.vendor.removed
```

The parser extracts the package names and uses them to mark entries returned by
the `-f` listing as disabled.

Minimum Android version: Android 1.0 (API 1). The `-d` flag is not available on
every vendor build.

Package listing failure output can include:

```text
Error: java.lang.SecurityException: Shell does not have permission
Exception occurred while executing 'list':
java.lang.IllegalArgumentException: Unknown option
```

The application rejects the listing when stdout contains `failure` or
`exception`, or when the process exits with a non-zero code.

## Process Table

### `adb -s <serial> shell top -b -n 1 -o PID,%CPU,RES,TIME+,ARGS -s 2`

Reads one batch-mode process snapshot.

Example output:

```text
Tasks: 550 total,   2 running, 548 sleeping,   0 stopped,   0 zombie
Mem:      5.5G total,      5.2G used,    300M free
800%cpu  62%user   2%nice  29%sys 707%idle   0%iow   0%irq   3%sirq   0%host
  PID %CPU  RES TIME+ ARGS
 1604  8.0 284M 1:04.08 system_server
 2201 25.3 120M 00:12.45 com.example.app
   42  0.0 12.5M 00:00.01 /system/bin/surfaceflinger --flag value
```

Possible variations:

- Column order can differ.
- The sorted column can be wrapped in brackets, for example `PID[%CPU]`.
- Memory can use suffixes such as `B`, `K`, `KB`, `KiB`, `M`, `MB`, `MiB`,
  `G`, `GB`, `GiB`, `T`, `TB`, or `TiB`.
- Summary lines appear before the table.
- A device can print a table without a recognized header.
- A vendor build can reject the `-o` or `-s` flags.
- Example unsupported output: `top: invalid option -- o`.

The parser locates the header, maps the PID, CPU, resident memory, CPU time, and
command columns, and ignores malformed rows. If no process rows are parsed, the
snapshot is rejected as malformed rather than shown as an empty list.

Minimum Android version: Android 8.0 (API 26).
Vendor images and older devices may use a different `top` implementation.

## Doze Whitelist

### `adb -s <serial> shell dumpsys deviceidle whitelist`

Reads packages exempt from Doze restrictions.

Example output:

```text
system,com.android.shell
system-excidle,com.google.android.gms
user,com.example.chat
com.example.legacy
```

Possible variations:

- A line can be only a package name.
- A line can have a trailing user ID, for example
  `user,com.whatsapp,10305`.
- The source can be `system`, `system-excidle`, `system-ui`, `user`, or an
  unrecognized value.
- Empty output is valid and means no entries were returned.
- Example unsupported output: `Can't find service: deviceidle`. This is reported
  as an unsupported command instead of an empty whitelist.

The parser validates package names, maps known sources, and ignores malformed
lines. Unsupported command or service responses are rejected before parsing.

Minimum Android version: Android 7.0 (API 24) for the
`dumpsys deviceidle whitelist` command.

### `adb -s <serial> shell dumpsys deviceidle whitelist +<package>`

Adds one package to the Doze whitelist.

Possible output:

```text
Added: com.example.chat
```

Possible failure output:

```text
Unknown package: com.example.chat
```

The command is successful only when it exits with code 0 and stdout does not
contain `unknown package`. Diagnostics from stderr are included in the error
message.

Minimum Android version: Android 7.0 (API 24).

### `adb -s <serial> shell dumpsys deviceidle whitelist -<package>`

Removes one package from the Doze whitelist.

Possible output:

```text
Removed: com.example.chat
```

Possible failure output:

```text
Unknown package: com.example.chat
```

Validation matches the add command.

Minimum Android version: Android 7.0 (API 24).

## Background Execution

### `adb -s <serial> shell cmd appops query-op RUN_ANY_IN_BACKGROUND ignore`

Lists packages whose background execution operation is set to `ignore`.

Example output:

```text
com.example.chat
com.example.mail
```

Possible outputs:

- Empty output when no package is restricted.
- `No operations.`
- Vendor diagnostics on stderr.
- `Error: ...` or `java.lang.SecurityException: ...` on stdout.
- Example unsupported output: `/system/bin/sh: cmd: not found`.

The parser keeps only valid package names, so diagnostic lines are ignored.
Unsupported command responses are rejected before parsing. The
command is successful only when it exits with code 0, stderr contains neither
`error` nor `exception`, and stdout does not start with `error`, `exception`, or
`java.lang.`.

Minimum Android version: Android 9.0 (API 28) for the `cmd appops`
`RUN_ANY_IN_BACKGROUND` commands.

### `adb -s <serial> shell cmd appops set <package> RUN_ANY_IN_BACKGROUND allow`

Allows background execution for one package.

Typical output is empty.

Possible failure output:

```text
Error: Unknown package: com.example
java.lang.SecurityException: ...
```

Validation matches the query command.

Minimum Android version: Android 9.0 (API 28).

### `adb -s <serial> shell cmd appops set <package> RUN_ANY_IN_BACKGROUND ignore`

Restricts background execution for one package.

Typical output is empty.

Possible failure output:

```text
Error: Unknown package: com.example
java.lang.SecurityException: ...
```

Validation matches the query command.

Minimum Android version: Android 9.0 (API 28).

## Application State Changes

These commands run only after `get-state` reports `device`.

### `adb -s <serial> shell pm disable-user --user 0 <package>`

Disables a package for user 0.

Possible output:

```text
Package com.example.chat new state: disabled-user
```

Possible failure output:

```text
Exception occurred while executing 'disable-user':
java.lang.SecurityException: Cannot disable system packages.
Error: java.lang.IllegalArgumentException: Unknown package
```

The application rejects the command when `pm` reports `failure` or `exception`,
or when the exit code is non-zero. Permission failures are reported separately
from generic command failures.

Minimum Android version: Android 4.2 (API 17) for `--user`. The `disable-user`
subcommand and its output wording vary on vendor builds.

### `adb -s <serial> shell pm enable --user 0 <package>`

Enables a package for user 0.

Possible output:

```text
Package com.example.chat new state: enabled
```

Possible failure output:

```text
Exception occurred while executing 'enable':
java.lang.SecurityException: ...
Error: java.lang.IllegalArgumentException: Unknown package
```

Validation matches `pm disable-user`.

Minimum Android version: Android 4.2 (API 17) for `--user`. The `enable`
subcommand and its output wording vary on vendor builds.

### `adb -s <serial> shell pm uninstall --user 0 <package>`

Uninstalls a package for user 0. Other users and the system image are not
modified by this command.

Possible output:

```text
Success
```

Possible failure output:

```text
Failure [DELETE_FAILED_INTERNAL_ERROR]
Failure [not installed for 0]
Exception occurred while executing 'uninstall':
java.lang.SecurityException: ...
```

The command is successful only when stdout is `Success` and no failure is
reported.

Minimum Android version: Android 5.0 (API 21) for reliable `--user` removal.
Some Android 4.2-4.4 builds accept the same command.

## Application Labels

### `adb -s <serial> pull <remote-apk> <local-apk>`

Copies the base APK to a local temporary file so the application label can be
read without running device-side tools.

Example output:

```text
/data/app/~~abc==/com.example-1/base.apk: 1 file pulled, 12.3 MB/s (12345678 bytes in 0.950s)
```

Possible failure output:

```text
adb: error: failed to stat remote object '/data/app/com.example/base.apk': No such file or directory
adb: error: cannot create file/directory 'local.apk': No such file or directory
```

The pull is successful only when it exits with code 0. The local file is
deleted after the label is read or the read fails. A missing or unreadable
label is not an error; the application falls back to the package name.

Minimum Android version: not applicable. This is an ADB host command.
