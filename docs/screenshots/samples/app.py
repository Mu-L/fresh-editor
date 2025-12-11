from dataclasses import dataclass
from typing import List, Optional

@dataclass
class Task:
    title: str
    completed: bool = False
    priority: int = 0

class TodoList:
    def __init__(self):
        self.tasks: List[Task] = []

    def add(self, title: str, priority: int = 0) -> Task:
        task = Task(title=title, priority=priority)
        self.tasks.append(task)
        return task

    def complete(self, index: int) -> Optional[Task]:
        if 0 <= index < len(self.tasks):
            self.tasks[index].completed = True
            return self.tasks[index]
        return None

if __name__ == "__main__":
    todo = TodoList()
    todo.add("Learn Fresh editor", priority=1)
    todo.add("Write some code")
    print(f"Tasks: {len(todo.tasks)}")
