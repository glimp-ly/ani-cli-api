# ani-cli-api

## Descripcion y objetivo
Este proyecto busca crear una api que obtenga datos de la pagina [animeav1](https://animeav1.com), en un json, obtien la busqueda, a un anime especifico, un capitulo de ese anime, y se reproduciran desde un cliente [ani-cli-es](https://github.com/glimp-ly/ani-cli-es) (aun en desarrollo). De esta manera la api controla las solicitudes a la pagina y ani-cli-es se encarga de mostrar loscapitulos y logica de uso.

Hasta el momento ya se puede obtener tanto los animes por criterios buscados y los episodios de una anime especifico, el endpoint encargado de la obtencion de las fuentes de video aun se encuentra en desarrollo.

## TO-DO
- [x] Recibir respuestas de los animes por criterio de busqueda en el nombre.
- [x] Recibir respuestas de los episodios de un anime especifico.
- [x] Recibir las fuentes de un capitulo especifico.
- [ ] Permitir al usuario agregar fuentes propias.
- [ ] Guardar previas conexiones para optimizar tiempos de carga.

## Contribuciones
Puedes realizar contribuciones haciendo el fork y la pullrequest, ayuda al desarrollo del proyecto.