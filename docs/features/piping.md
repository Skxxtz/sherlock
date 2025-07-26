# Piping
Piping is used to take the output of one program and use it as input for another one.<br>

## Piping in Sherlock
In Sherlock, you can pipe content into and out of the program. The input can be in either raw text or json format.<br>

### Raw Text
Using raw text, the content will be split into lines and each line will be assigned to a row in the application. Alternatively, the text can also be displayed as raw text using the `--display-raw` flag. This will keep the text formatting such as tabs and lines. You could use this to display ASCII art inside of Sherlock.

### Json format
When providing json data, the data should follow this formatting:
```json
{
  "settings": [],
  "elements": [
    {
      "title": "string",
      "description": "string",
      "icon": "string",
      "icon_size": 64,
      "result": "string",
      "method": "string",
      "field": "string",
      "hidden": {
        "key1": "value",
        "key2": "value"
      }
    }
  ]
}
```
#### Fields
All fields are optional.<br>
| Name | Explanation |
| -------------- | --------------- |
| `title` | This sets the title object where you would normally see application names. |
| `description` | This sets the description where you would normally see launcher names. |
| `icon` | This sets the icon name for the tile. |
| `icon_size` | Sets the size of the icon. |
| `result` | Sets the content that should be handled as result. Defaults to `title`. |
| `binary` | The binary will be evaluated as an image. If it succeeds, the icon will be replaced with it. |
| `field` | Specifies the field which hidden field should be used as the output. |
| `method` | Sets the action on how to handle the output to either `print` or `copy`. Will default to `print` |
| `hidden` | This is a set of hidden elements, that will not be shown but can be accessed as a result using the `--field` flag. |
| `exit` | This bool specifies whether Sherlock should exit after pressing `<RETURN>` or stay active |


### Flags
| Name | Explanation |
| -------------- | --------------- |
| `--display-raw` | Makes Sherlock display the content in a single text field, retaining formatting. |
| `--center` | Centers the text when using `--display-raw`. |
| `--field` | When using json input, selects a field which should be used as the output. Will be overwritten by the individual fields. |
| `--method` | Sets the option on how to handle the output. Can be either `print` or `copy`. It will default to print which will print the output to std-out.  |

