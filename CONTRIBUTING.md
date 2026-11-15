# Cómo contribuir

Gracias por sumarte. Issues y PRs en español o inglés.

## Antes de empezar

1. Buscá si ya hay un issue. Si no, abrí uno y contá qué querés hacer antes de un PR grande.
2. Issues fáciles: etiqueta `good first issue`.

## Flujo

1. Hacé un fork y una rama: `feat/editor-audio`, `fix/progreso-playlist`.
2. Seguí los pasos de *Desarrollo* del README.
3. Antes del PR corré:
   ```bash
   npm run build
   cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
   ```
4. Commits cortos y descriptivos. Un PR, un tema.

## Reglas del proyecto

- **Código propio.** No copies código, textos, íconos ni capturas de otros programas (libres o comerciales con licencia incompatible). Reimplementá la función.
- **Nada de evadir DRM** ni protecciones anticopia. No se aceptan PRs para eso.
- **Ejemplos limpios.** En docs, tests e issues usá contenido propio o con licencia libre, nunca música o películas con copyright.
- **Nada de credenciales.** La app nunca pide ni guarda contraseñas.
- La lógica de descarga vive en yt-dlp: los problemas de "este sitio no funciona" van a [yt-dlp](https://github.com/yt-dlp/yt-dlp/issues), salvo que sea un error nuestro al llamarlo.

## Licencia

Al contribuir aceptás que tu aporte se publica bajo GPL-3.0-or-later.

## Conducta

Este proyecto sigue el [Contributor Covenant 2.1](https://www.contributor-covenant.org/es/version/2/1/code_of_conduct/). Sé amable; los reportes van a los mantenedores por email o mensaje privado.
