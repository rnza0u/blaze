FROM nginx:1.25.4-alpine


COPY conf/nginx.conf /etc/nginx/nginx.conf
RUN rm /etc/nginx/conf.d/*

COPY build /usr/share/nginx/html