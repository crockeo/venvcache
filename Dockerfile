ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

COPY ./scripts/install_sqlite3.sh /tmp/install_sqlite3.sh
RUN : \
    && chmod +x /tmp/install_sqlite3.sh \
    && /tmp/install_sqlite3.sh \
    && rm -f /tmp/install_sqlite3.sh
