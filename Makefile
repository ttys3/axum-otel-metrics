doc/dev:
	cargo doc --no-deps --document-private-items --open

doc:
	cargo doc --no-deps --open

changelog:
	# pacman -S git-cliff
	# brew install git-cliff
	# cargo binstall git-cliff
	#git-chglog > CHANGELOG.md
	git cliff | tee CHANGELOG.md | bat -l markdown -P
	git add ./CHANGELOG.md && git commit -m "docs: update CHANGELOG.md"
