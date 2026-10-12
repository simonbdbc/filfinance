export class RecentlyPublishedIds {
  constructor(limit) {
    this.limit = limit;
    this.ids = new Set();
  }

  has(id) {
    return Boolean(id) && this.ids.has(id);
  }

  remember(id) {
    if (!id) {
      return;
    }
    this.ids.add(id);
    while (this.ids.size > this.limit) {
      const oldest = this.ids.values().next().value;
      this.ids.delete(oldest);
    }
  }

  rememberAll(events) {
    for (const event of events) {
      this.remember(event.external_id);
    }
  }
}
