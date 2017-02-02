# coding: utf-8
lib = File.expand_path('../lib', __FILE__)
$LOAD_PATH.unshift(lib) unless $LOAD_PATH.include?(lib)
require 'hokaido/version'

Gem::Specification.new do |spec|
  spec.name          = 'hokaido'
  spec.version       = Hokaido::VERSION
  spec.authors       = %w(ursm hibariya)
  spec.email         = %w(ursm@ursm.jp celluloid.key@gmail.com)
  spec.summary       = 'HND✈CTS'
  spec.homepage      = 'https://github.com/ursmhbry/hokaido'
  spec.license       = 'MIT'

  spec.files         = `git ls-files -z`.split("\x0")
  spec.executables   = spec.files.grep(%r{^bin/}) { |f| File.basename(f) }
  spec.test_files    = spec.files.grep(%r{^(test|spec|features)/})
  spec.require_paths = ['lib']

  spec.add_runtime_dependency 'celluloid'
  spec.add_runtime_dependency 'ruby-terminfo'
  spec.add_runtime_dependency 'thor'

  spec.add_development_dependency 'bundler', '~> 1.7'
  spec.add_development_dependency 'rake', '~> 10.0'
end
