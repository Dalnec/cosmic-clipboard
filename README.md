# Copyous COSMIC

**Copyous COSMIC** es la próxima generación y reescritura nativa del portapapeles "Copyous" (originalmente una extensión de GNOME Shell), diseñada específicamente desde cero en **Rust** para integrarse de forma perfecta con el futuro entorno de escritorio **Pop!_OS COSMIC Epoch**.

## Características Principales

* **Reescritura Nativa en Rust:** Mayor rendimiento, seguridad en el manejo de memoria y mínima latencia en el sistema.
* **Interfaz Estable (Iced):** Utiliza el framework gráfico `Iced` bajo el cual está construido el propio entorno COSMIC, prometiendo compatibilidad total de estilos sin depender de las complejas dependencias y roturas de GNOME GJS.
* **Persistencia Inteligente de SQLite:** Guarda tus recortes localmente y sin cuellos de botella empleando la librería local `rusqlite`.
* **Daemon Asíncrono de Wayland:** Supervisa activamente copias del entorno gráfico y en segundo plano sin ralentizar tu equipo apoyado sobre un entorno veloz con `Tokio`.

---

## Manual de Uso e Instalación

### Requisitos Previos

Asegúrate de tener un entorno de desarrollo de Rust instalado en tu sistema. Si no lo tienes, instala `cargo` vía `rustup`:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Compilación Básica

1. **Clona o navega** a este directorio desde la terminal:
   ```bash
   cd ~/dev/portapapeles/cosmic-copyous
   ```

2. **Compila la rama de fondo (Backend daemon):**
   ```bash
   cargo build --release
   ```

3. **Inicia el proceso para probar el motor logístico:**
   ```bash
   cargo run
   ```

*(Nota: De momento, `cargo run` inicia el proceso de fondo asincrónico `main.rs` encargado de monitorear pasivamente el portapapeles y actualizar la base de datos)*.

### ¿Cómo interactuar con el Daemon?

Una vez iniciado `cargo run`, el sistema comenzará a ejecutarse en un bucle en consola indicando: `"Iniciando hilo para escuchar cambios..."`.
Cualquier cosa que copies con `Ctrl+C` en tu navegador u otro programa de tu ecosistema de ventanas será interceptado y verás en consola un mensaje como:
```
> Nuevo elemento interceptado: Texto de prueba copiado...
```
Automáticamente, sin más intervención, **este texto será registrado en tu base de datos SQLite**.

### Almacenamiento e Historial

Tu historial histórico (compatible a nivel de esquema de tabla con la base de datos clásica de GNOME Copyous) ahora reside y se aísla de forma segura en las carpetas de configuración de este nuevo applet de COSMIC, concretamente en:
* `/home/TU_USUARIO/.local/share/cosmic/com.boerdereinar.copyous/clipboard.db`

Si alguna vez deseas borrar manualmente todo tu historial, puedes eliminar ese pequeño archivo `.db` de las carpetas de COSMIC y la aplicación lo regenerará la próxima vez que se ejecute.

### Arquitectura Técnica del Entorno
* **`src/db/`:** Maneja conexiones e inserciones automáticas hacia SQLite mediante `tokio` threads.
* **`src/clipboard/`:** Instancía módulos de escucha con `arboard` sobre Wayland / X11.
* **`src/ui/`:** Raíz y componentes para armar las vistas del portapapeles listas para vincular en el loop madre (`mod.rs` centraliza el estilo Elm clásico de UI: vistas, estado, mensajes).
