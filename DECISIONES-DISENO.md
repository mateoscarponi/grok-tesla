# Decisiones de diseño

Por qué el repo está armado así. Si cambiás algo de esto, anotalo acá con fecha.

1. **El README es la puerta de entrada y se sostiene solo.** (2026-11-22)
   Grok en el auto puede no seguir todos los links. Por eso el README tiene lo esencial (identidad, reglas de manejo, roles en resumen) y los demás archivos amplían. Si Grok solo lee el README, igual funciona bien.

2. **Links a la versión "raw" de cada archivo.** (2026-11-22)
   Las URLs `raw.githubusercontent.com` devuelven texto plano, más fácil de leer para un agente que la página de GitHub.

3. **Repo público, sin datos privados.** (2026-11-22)
   Grok solo puede abrir links públicos. Todo lo que se escribe acá lo puede leer cualquiera.

4. **Español y voz primero.** (2026-11-22)
   Mateo habla en español mientras maneja. Todas las reglas priorizan respuestas cortas y escuchables.

5. **La seguridad al manejar está por encima de cualquier rol.** (2026-11-22)

6. **Cambios con trazabilidad.** (2026-11-22)
   Cada cambio va al `CHANGELOG.md`; cada prueba de prompts, a `experiments/`.
