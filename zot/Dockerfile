FROM ubuntu

RUN apt update; apt install -y wget curl
RUN wget -O /usr/bin/zot https://github.com/project-zot/zot/releases/download/v2.0.4/zot-linux-amd64; \
    chmod +x /usr/bin/zot; \
    chown root:root /usr/bin/zot; \
    mkdir /etc/zot/

VOLUME /var/lib/zot

ADD config.json /etc/zot/config.json

CMD [ "zot", "serve", "/etc/zot/config.json" ]