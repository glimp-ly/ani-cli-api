# ani-cli-api

## Descripcion y objetivo
Este proyecto busca crear una api que obtenga datos de la pagina [animeav1](https://animeav1.com), en un json, obtien la busqueda, a un anime especifico, un capitulo de ese anime, y se reproduciran desde un cliente [ani-cli-es](https://github.com/glimp-ly/ani-cli-es) (aun en desarrollo). De esta manera la api controla las solicitudes a la pagina y ani-cli-es se encarga de mostrar loscapitulos y logica de uso.

Se puede obtener tanto los animes por criterios buscados, los episodios de una anime especifico y las fuente de un episodio elegido.

## Dependencias necesarias
Este proyecto requiere:

- `chromedriver`
- `chromium-browser` **o** `chromium` **o** `google-chrome`

En Debian/Ubuntu:

```bash
# Instalar Google Chrome
wget https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
sudo apt install ./google-chrome-stable_current_amd64.deb -y

# Instalar Chromedriver
wget https://storage.googleapis.com/chrome-for-testing-public/139.0.7258.127/linux64/chromedriver-linux64.zip
unzip chromedriver-linux64.zip
sudo mv chromedriver-linux64/chromedriver /usr/local/bin/
sudo chmod +x /usr/local/bin/chromedriver
```

En Arch:
```bash
sudo pacman -S chromium chromedriver
```

## TO-DO
- [x] Recibir respuestas de los animes por criterio de busqueda en el nombre.
- [x] Recibir respuestas de los episodios de un anime especifico.
- [x] Recibir las fuentes de un capitulo especifico.
- [ ] Permitir al usuario agregar fuentes propias.
- [ ] Guardar previas conexiones para optimizar tiempos de carga.

## Contribuciones
Puedes realizar contribuciones haciendo el fork y la pullrequest, ayuda al desarrollo del proyecto.