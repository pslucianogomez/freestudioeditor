# Cómo contribuir

Gracias por sumarte. Issues y PRs en español o inglés.

## Antes de empezar

1. Buscá si ya hay un issue. Si no, abrí uno y contá qué querés hacer antes de un PR grande.
2. Issues fáciles: etiqueta `good first issue`.

## Flujo de trabajo

Usamos [GitHub Flow](https://docs.github.com/es/get-started/using-github/github-flow): `main` siempre funciona y **todo cambio entra por un Pull Request revisado**. Nadie sube directo a `main`, ni siquiera los mantenedores.

```
main ───●──────────●──────────●───  (las versiones son etiquetas: v0.1.0, v0.2.0…)
         \        /  \       /
          feat/x       fix/y        ← ramas de tu fork, una por tema
```

1. **Fork** del repositorio (botón *Fork* arriba a la derecha) y clonalo.
2. **Rama** desde `main` actualizado, con un nombre que diga qué hace:
   - `feat/…` función nueva (`feat/editor-audio-fundidos`)
   - `fix/…` corrección (`fix/progreso-playlist`)
   - `docs/…` documentación
3. Seguí los pasos de *Desarrollo* del README y hacé tus cambios.
4. **Antes de abrir el PR** corré lo mismo que corre el CI:
   ```bash
   npm run build
   cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
   ```
5. **Abrí el PR** hacia `main` y completá la plantilla. Si resuelve un issue, escribí `Closes #123`.
6. **Revisión:** el CI tiene que pasar en verde y un mantenedor tiene que aprobarlo. Si te piden cambios, subilos a la misma rama: el PR se actualiza solo.
7. **Merge:** el mantenedor lo une con *Squash and merge* (todo el PR queda como un solo commit en `main`).

Consejos:
- **Un PR, un tema.** Varios cambios chicos se revisan más rápido que uno gigante.
- Si es tu primer PR, el CI espera a que un mantenedor lo autorice: es normal.
- Para cambios grandes, abrí antes un issue y charlemos el enfoque.

## Revisión de código

Los PRs se asignan solos a los mantenedores (ver [`.github/CODEOWNERS`](.github/CODEOWNERS)). Al revisar se mira:

- Que haga lo que dice y no rompa otras herramientas.
- Que tenga pruebas si toca lógica del núcleo (`src-tauri/src/`). Si arma un comando nuevo de FFmpeg, sumá un caso en `real_ffmpeg.rs`.
- Que respete las reglas del proyecto (abajo).
- Textos de la interfaz claros y cortos, con voseo como el resto de la app ("elegí", "pegá").

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
