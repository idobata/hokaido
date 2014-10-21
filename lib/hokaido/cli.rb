require 'io/console'
require 'pty'
require 'socket'
require 'terminfo'
require 'thor'
require 'thread'

module Hokaido
  class CLI < Thor
    class_option :host, aliases: :h, default: '0.0.0.0'
    class_option :port, aliases: :p, default: 4423

    desc :broadcast, 'Broadcast a session'
    def broadcast(command = ENV['SHELL'])
      pty_out, pty_in, pid = *PTY.getpty(command)
      nonbloq              = Queue.new

      trap :SIGWINCH do
        TermInfo.tiocswinsz pty_in, *TermInfo.screen_size
      end

      Thread.abort_on_exception = true

      Thread.start do
        server = TCPSocket.open(*options.values_at(:host, :port))
        server.puts 'write'

        while c = nonbloq.deq
          server.putc c
        end
      end

      Thread.start do
        while c = pty_out.getc
          $stdout.putc c
          nonbloq.enq c
        end
      end

      Thread.start do
        TermInfo.tiocswinsz pty_in, *TermInfo.screen_size

        while c = $stdin.getch
          pty_in.putc c
        end
      end

      Process.waitpid pid
    end

    desc :server, 'Start server'
    def server
      queue  = Queue.new
      server = TCPServer.open(*options.values_at(:host, :port))

      loop do
        Thread.start server.accept do |client|
          case client.gets.chomp
          when 'write'
            client.puts ':)'

            while c = client.getc
              queue.enq c
            end
          when 'read'
            client.puts '=)'

            while c = queue.deq
              client.putc c
            end
          else
            client.puts ':('
          end

          client.close
        end
      end
    rescue Interrupt
      server.close

      exit
    end

    desc :watch, 'Watch a session'
    def watch
      server = TCPSocket.open(*options.values_at(:host, :port))
      server.puts 'read'

      while c = server.getc
        $stdout.putc c
      end
    rescue Interrupt
      server.close

      exit
    end
  end
end
