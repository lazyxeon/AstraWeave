# Community

Welcome to the AstraWeave community! This page lists resources for connecting with other developers, getting help, and contributing to the project.

## Official Channels

### GitHub

The primary home for AstraWeave development:

| Resource | Description |
|----------|-------------|
| [Repository](https://github.com/astraweave/astraweave) | Source code, issues, and pull requests |
| [Discussions](https://github.com/astraweave/astraweave/discussions) | Q&A, ideas, and general discussion |
| [Issues](https://github.com/astraweave/astraweave/issues) | Bug reports and feature requests |
| [Projects](https://github.com/orgs/astraweave/projects) | Development roadmap and tracking |

### Discord

Join our Discord server for real-time chat:

- **#general** - Introduce yourself and chat with the community
- **#help** - Get help with AstraWeave questions
- **#showcase** - Share your projects and creations
- **#ai-discussion** - Discuss AI/LLM integration topics
- **#development** - Engine development discussion
- **#announcements** - Official news and updates

### Social Media

- **Twitter/X**: [@AstraWeaveEngine](https://twitter.com/astraweave) - News and updates
- **YouTube**: [AstraWeave](https://youtube.com/@astraweave) - Tutorials and devlogs
- **Reddit**: [r/astraweave](https://reddit.com/r/astraweave) - Community discussions

## Getting Help

### Before Asking

1. **Search existing resources**:
   - [Documentation](../index.md)
   - [FAQ](faq.md)
   - [Troubleshooting](troubleshooting.md)
   - [GitHub Issues](https://github.com/astraweave/astraweave/issues)
   - [GitHub Discussions](https://github.com/astraweave/astraweave/discussions)

2. **Prepare your question**:
   - Describe what you're trying to do
   - Show what you've tried
   - Include relevant code snippets
   - Share error messages

### Where to Ask

| Question Type | Best Channel |
|--------------|--------------|
| Quick questions | Discord #help |
| Detailed technical questions | GitHub Discussions |
| Bug reports | GitHub Issues |
| Feature requests | GitHub Issues or Discussions |
| Security issues | security@astraweave.dev (private) |

### How to Ask Good Questions

```markdown
## What I'm trying to do
I want to create an AI companion that follows the player and helps in combat.

## What I've tried
```rust
// My current code
fn companion_follow(
    companion: Query<&mut Transform, With<Companion>>,
    player: Query<&Transform, With<Player>>,
) {
    // This doesn't work as expected
}
```

## What happens
The companion doesn't move at all.

## What I expected
The companion should move toward the player.

## Environment
- AstraWeave 0.1.0
- Windows 11
```

## Contributing

### Ways to Contribute

Everyone can contribute, regardless of experience level:

| Contribution | Description |
|--------------|-------------|
| **Bug Reports** | Found a bug? Report it on GitHub Issues |
| **Documentation** | Improve or add documentation |
| **Code** | Fix bugs or implement features |
| **Examples** | Create example projects |
| **Testing** | Test pre-release versions |
| **Translations** | Help translate documentation |
| **Community** | Help others in Discord/Discussions |

### Getting Started

1. **Read the contributing guide**: [Contributing](../dev/contributing.md)
2. **Set up your development environment**: [Building](../dev/building.md)
3. **Find a good first issue**:
   - Look for `good first issue` labels on GitHub
   - Check the `help wanted` label for more challenging tasks

### Code Contribution Workflow

```bash
# 1. Fork and clone
git clone https://github.com/YOUR_USERNAME/astraweave.git
cd astraweave

# 2. Create a branch
git checkout -b feature/my-feature

# 3. Make changes
# ... edit files ...

# 4. Run tests
cargo test --all

# 5. Commit
git commit -am "feat: add my feature"

# 6. Push and create PR
git push origin feature/my-feature
```

### Pull Request Guidelines

- **Keep PRs focused**: One feature or fix per PR
- **Write tests**: Cover new functionality with tests
- **Update docs**: Add documentation for new features
- **Follow style**: Match existing code style
- **Be patient**: Reviews take time

## Showcase

Share your AstraWeave projects!

### How to Submit

1. **Discord**: Post in #showcase with:
   - Project name
   - Brief description
   - Screenshots or video
   - Link (if public)

2. **GitHub Discussions**: Create a post in the "Show and Tell" category

### Featured Projects

Projects using AstraWeave:

| Project | Description |
|---------|-------------|
| [hello_companion](../examples/hello-companion.md) | Official demo showcasing AI companion systems |
| [adaptive-boss](../examples/adaptive-boss.md) | Boss AI that learns and adapts to player strategies |

Have a project to share? Post in [#showcase on Discord](https://discord.gg/astraweave) or open a Discussion on GitHub!

## Events

### Community Events

- **Monthly Showcase**: First Friday of each month
- **AI Game Jam**: Quarterly jam focusing on AI-native games
- **Office Hours**: Weekly Q&A with maintainers

### Conferences

AstraWeave may be presented at:
- GDC (Game Developers Conference)
- RustConf
- Various game development meetups

## Governance

### Project Leadership

| Role | Responsibility |
|------|----------------|
| **Core Team** | Major decisions, releases, roadmap |
| **Maintainers** | Code review, issue triage |
| **Contributors** | Feature development, bug fixes |

### Decision Making

- **Minor changes**: Maintainer approval
- **Major changes**: Core team discussion
- **Breaking changes**: RFC process with community input

### Code of Conduct

We follow a Code of Conduct to ensure a welcoming community:

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn
- No harassment or discrimination

Full Code of Conduct: [CODE_OF_CONDUCT.md](https://github.com/astraweave/astraweave/blob/main/CODE_OF_CONDUCT.md)

## Learning Resources

### Official Resources

- [Documentation](../index.md) - Complete documentation
- [Examples](../examples/index.md) - Working code examples
- [API Reference](https://docs.rs/astraweave) - Generated API docs

### Community Resources

- **Tutorials**: Community-created tutorials (see Discord pinned messages)
- **Templates**: Starter templates for common project types
- **Libraries**: Community extensions and plugins

### Recommended Learning Path

1. **Getting Started**: [Installation](../getting-started/installation.md)
2. **First Project**: [Building Your First Game](../game-dev/first-game.md)
3. **Core Concepts**: [Architecture Overview](../architecture/overview.md)
4. **AI Features**: [AI System](../core-systems/ai/index.md)
5. **Advanced Topics**: Explore specific system documentation

## Staying Updated

### Release Notes

- Check [GitHub Releases](https://github.com/astraweave/astraweave/releases)
- Watch the repository for release notifications

### Changelog

Each release includes a detailed changelog covering:
- New features
- Bug fixes
- Breaking changes
- Migration guides

### Roadmap

See [Roadmap](roadmap.md) for upcoming features and long-term plans.

## Sponsorship

### Supporting Development

If AstraWeave helps you, consider supporting its development:

- **GitHub Sponsors**: Support individual maintainers
- **Corporate Sponsorship**: Contact sponsorship@astraweave.dev

### Sponsors

Thank you to our sponsors! Interested in sponsoring AstraWeave? Contact sponsorship@astraweave.dev to learn about sponsorship tiers and benefits.

## Contact

| Purpose | Contact |
|---------|---------|
| General inquiries | hello@astraweave.dev |
| Security issues | security@astraweave.dev |
| Sponsorship | sponsorship@astraweave.dev |
| Press | press@astraweave.dev |

## Related Documentation

- [Contributing](../dev/contributing.md) - Contribution guidelines
- [Code Style](../dev/code-style.md) - Coding standards
- [Roadmap](roadmap.md) - Future plans
- [FAQ](faq.md) - Frequently asked questions
