import mqttd.server;
import mqttd.stream;
import std.typecons;
import std.string;
import core.memory;


struct Span {
    ubyte* ptr;
    long size;
}

extern(C) {
    void rust_new_message(void* context, in Span bytes);
    void rust_disconnect(void* context);


    void startMqttServer(bool useCache) {
        GC.disable;
        gServer = typeof(gServer)(useCache ? Yes.useCache : No.useCache);
    }

    Subscriber* newDlangSubscriber(void* connection) {
        return new Subscriber(connection);
    }

    Span getWriteableBuffer(Subscriber* subscriber) {
        return arrayToSpan(subscriber._stream.buffer());
    }

    const(char*) handleMessages(Subscriber* subscriber, long numBytesRead ) {
        try {
            subscriber._stream.handleMessages(numBytesRead, gServer, subscriber._subscriber);
            return null;
        } catch(Throwable t) {
            return t.msg.toStringz();
        }
    }
}

private inout(Span) arrayToSpan(inout(ubyte)[] bytes) {
    return inout(Span)(cast(inout(ubyte)*)bytes.ptr, bytes.length);
}


private struct Subscriber {
    this(void* connection) {
        _stream = MqttStream(512 * 1024);
        _subscriber = SubscriberImpl(connection);
    }

    static struct SubscriberImpl {

        void newMessage(in ubyte[] bytes) {
            assert(bytes.length > 0);
            rust_new_message(_rustConnection, arrayToSpan(bytes));
        }

        void disconnect() {
            rust_disconnect(_rustConnection);
        }

        void* _rustConnection;
    }

    extern(C++) {
        Span getWriteableBuffer() {
            return arrayToSpan(_stream.buffer());
        }

        const(char)* handleMessages(long numBytesRead) {
            try {
                _stream.handleMessages(numBytesRead, gServer, _subscriber);
                return null;
            } catch(Throwable t) {
                return t.msg.toStringz();
            }
        }
    }

private:

    MqttStream _stream;
    SubscriberImpl _subscriber;
}


private __gshared MqttServer!(Subscriber.SubscriberImpl) gServer;
