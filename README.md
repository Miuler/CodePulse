
# Statistic

La aplicacion lee todos los archivos .java de un directorio y cuenta los metodos y variables.
Lee todos los archivos que se encuentren en el directorio definido en la variable JAVA_SRC.
Si no existe la variable lee el archivo .env para sacar todas las variables faltantes desde ese archivo.


# Ejemplo de la salida del programa

```
  2025-09-04T00:01:43.203830Z  INFO statistic: STARTING
    at src/main.rs:21

  2025-09-04T00:01:49.866354Z  INFO statistic::statistic: 
┌───────────┬──────────┐
│ Metrica   ┆ Cantidad │
╞═══════════╪══════════╡
│ Metodos   ┆ 20459    │
├╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌┤
│ Variables ┆ 5149     │
└───────────┴──────────┘
    at src/statistic.rs:27
```

