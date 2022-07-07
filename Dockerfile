# create the build container to compile the program
FROM rust as build
WORKDIR /src
COPY . /src/
RUN git clone --depth 1 git://git.musl-libc.org/musl && \
    cd musl && \
    ./configure && make install && \
    cd ..
RUN rustup target add x86_64-unknown-linux-musl && \
    PATH="/usr/local/musl/bin:$PATH" cargo b -r --target x86_64-unknown-linux-musl

# create the execution container
FROM scratch
COPY --from=build /src/target/x86_64-unknown-linux-musl/release/genshin_check_in /server
EXPOSE 9000
CMD ["/server"]