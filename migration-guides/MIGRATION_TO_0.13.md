# Migrating to Yoleck 0.13

## Accessing the YoleckUi

`YoleckUi` is now a non-`Send` resource, which means it can no longer be accessed as a regular `Res`/`ResMut`. It must now be accessed as `NonSend`/`NonSendMut`.

Hopefully once https://github.com/emilk/egui/issues/3148 is fixed (and gets in to bevy_egui) this can be changed back.
